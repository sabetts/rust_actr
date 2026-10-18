//! Event-hook (`.lisp` `add-pre/post-event-hook`) regression cases.
//!
//! ACT-R's `:SCOPE :HOOK` stubs run the model through an arbitrary-Lisp
//! filter. This module reproduces each filter's *effect* — not its Lisp — with
//! a small closed IR ([`HookIr`]) executed by one [`EventHook`] (`IrHook`)
//! against the engine. Two output modes:
//!
//! * [`OutputMode::Project`] — the filter builds a list from `format-event`
//!   and prints it (`model-output "~{…~}"`): the demo2-family projection.
//! * [`OutputMode::Segment`] — the filter only toggles `:v` (and may break),
//!   so the compared output is the engine's raw trace (`out_lines`): the
//!   `pcomp-issues` control family and the scheduled `:v`-window cases.
//!
//! The driver (models/seed/steps/device) is reused from an existing `.spec`;
//! [`HookIr::tail`] appends extra steps (e.g. the `run-n-events` that resumes
//! after a break). The expected output is the case's own `.ref`.
//!
//! See `PORTING-NOTES.md` #269 and `fixtures/regression/README.md`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use actr_engine::{run_model_steps_with_hook, EventHook, EventView, PreResult, Step, Verdict};

use super::{actr_dir, first_diff, load_spec_device, normalize, read_named_spec, refs_dir};

// ------------------------------------------------------------------ IR

/// How the collected items are joined (the stub's `model-output` format
/// string). `Newline` is `~{~a~^~%~}`; `NewlineDots` is `~{~a~^~%...~%~}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    Newline,
    NewlineDots,
}

/// Which output is compared: the filter's projected list, or the engine's raw
/// trace (the filter's `:v` gates decide what accumulates).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Project,
    Segment,
}

/// A predicate over an [`EventView`]; mirrors the `evt-*` tests the stub
/// filters write. Deliberately closed: anything richer is a hand-written
/// hook, not an IR case.
#[derive(Debug, Clone, Copy)]
pub enum Pred {
    /// `(eq (evt-action evt) 'x)`.
    Action(&'static str),
    /// `(eq (evt-module evt) 'x)`.
    Module(&'static str),
    /// `(eq (car (evt-params evt)) 'x)` for a buffer-taking action.
    Buffer(&'static str),
    /// `(eq (production-name (car (evt-params evt))) 'x)`.
    Production(&'static str),
    /// `(= (evt-mstime evt) n)`.
    MsEq(i64),
    /// `(< (evt-mstime evt) n)`.
    MsLt(i64),
    /// `(<= lo (evt-mstime evt) hi)`.
    MsBetween(i64, i64),
    /// The stub's `*…-trace-started*` / latch flag.
    Latch,
    All(&'static [Pred]),
    Any(&'static [Pred]),
    Not(&'static Pred),
}

/// One rule. For [`OutputMode::Project`] only `when`/`keep`/`set` matter
/// (first match wins, the stubs' `or` ordering). For [`OutputMode::Segment`]
/// every matching rule applies its `trace`/`set`/`stop_after` effect.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub when: Pred,
    /// Keep the event (Project); ignored for Segment.
    pub keep: bool,
    /// `Some(true)` sets the latch, `Some(false)` clears it.
    pub set: Option<bool>,
    /// `Some(b)` sets `:v` (`(sgp :v …)` inside the hook).
    pub trace: Option<bool>,
    /// Count matches; at the `n`th, withhold the event and stop the run.
    pub stop_after: Option<u64>,
}

/// One hook phase: the rules, plus the `:v` a phase sets when it is entered
/// (the `(sgp :v t)` between the stub's two hooks).
#[derive(Debug, Clone, Copy)]
pub struct Phase {
    pub on_enter_trace: Option<bool>,
    pub rules: &'static [Rule],
}

/// A whole hook case: the driver `.spec`, the phases, the output mode, and
/// (for [`OutputMode::Project`]) the projection formatting.
#[derive(Debug, Clone, Copy)]
pub struct HookIr {
    /// Name of the `.spec` whose models/steps/seed/device drive the run.
    pub driver: &'static str,
    pub phases: &'static [Phase],
    pub output: OutputMode,
    /// Push `"..."` on the first non-kept event after a kept group, deduped.
    pub else_dots: bool,
    /// Drop the final collected item (`butlast`).
    pub butlast: bool,
    pub join: Join,
}

fn eval(p: &Pred, ev: &EventView, latch: bool) -> bool {
    match p {
        Pred::Action(a) => ev.action.eq_ignore_ascii_case(a),
        Pred::Module(m) => ev.module.eq_ignore_ascii_case(m),
        Pred::Buffer(b) => ev
            .buffer
            .as_deref()
            .is_some_and(|x| x.eq_ignore_ascii_case(b)),
        Pred::Production(n) => ev
            .production
            .as_deref()
            .is_some_and(|x| x.eq_ignore_ascii_case(n)),
        Pred::MsEq(t) => ev.time_ms == *t,
        Pred::MsLt(t) => ev.time_ms < *t,
        Pred::MsBetween(lo, hi) => *lo <= ev.time_ms && ev.time_ms <= *hi,
        Pred::Latch => latch,
        Pred::All(ps) => ps.iter().all(|p| eval(p, ev, latch)),
        Pred::Any(ps) => ps.iter().any(|p| eval(p, ev, latch)),
        Pred::Not(p) => !eval(p, ev, latch),
    }
}

#[derive(Default)]
struct State {
    lines: Vec<String>,
    latch: bool,
    started: bool,
    last_dot: bool,
    phase: usize,
    counts: HashMap<(usize, usize), u64>,
    pending_trace: Option<bool>,
}

impl State {
    fn render(&self, ir: &HookIr) -> Vec<String> {
        let mut lines = self.lines.clone();
        if ir.butlast {
            lines.pop();
        }
        let joined = match ir.join {
            Join::Newline => lines.join("\n"),
            Join::NewlineDots => lines.join("\n...\n"),
        };
        normalize(&joined)
    }
}

struct IrHook {
    ir: HookIr,
    state: Rc<RefCell<State>>,
}

impl EventHook for IrHook {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if self.ir.output == OutputMode::Project {
            return PreResult::CONTINUE;
        }
        let mut st = self.state.borrow_mut();
        let phase = st.phase.min(self.ir.phases.len() - 1);
        let mut trace = st.pending_trace.take();
        let mut verdict = Verdict::Continue;
        for (i, rule) in self.ir.phases[phase].rules.iter().enumerate() {
            if eval(&rule.when, ev, st.latch) {
                if let Some(t) = rule.trace {
                    trace = Some(t);
                }
                if let Some(s) = rule.set {
                    st.latch = s;
                }
                if let Some(n) = rule.stop_after {
                    let c = st.counts.entry((phase, i)).or_insert(0);
                    *c += 1;
                    if *c >= n {
                        verdict = Verdict::Break;
                    }
                }
            }
        }
        if verdict == Verdict::Break && phase + 1 < self.ir.phases.len() {
            st.phase = phase + 1;
            st.pending_trace = self.ir.phases[st.phase].on_enter_trace;
        }
        PreResult { verdict, trace }
    }

    fn post(&mut self, line: &str, ev: &EventView) {
        if self.ir.output == OutputMode::Segment {
            return;
        }
        let mut st = self.state.borrow_mut();
        let phase = st.phase.min(self.ir.phases.len() - 1);
        let mut kept = false;
        for rule in self.ir.phases[phase].rules {
            if eval(&rule.when, ev, st.latch) {
                if let Some(v) = rule.set {
                    st.latch = v;
                }
                kept = rule.keep;
                break;
            }
        }
        if kept {
            st.lines.push(line.to_string());
            st.started = true;
            st.last_dot = false;
        } else if self.ir.else_dots && st.started && !st.last_dot {
            st.lines.push("...".to_string());
            st.last_dot = true;
        }
    }
}

/// Run one hook case: drive with `ir.driver`'s spec (plus `ir.tail`), collect
/// the projection or the gated trace, and compare to `<case>.ref` under
/// ACT-R's own comparator.
pub fn run_hook_case(case: &str, ir: &HookIr) -> Result<(), String> {
    let root = actr_dir();
    if !root.is_dir() {
        return Err(format!("ACT-R checkout not found at {}", root.display()));
    }
    let spec = read_named_spec(ir.driver)?;
    if spec.steps.iter().any(|s| matches!(s, Step::Command { .. })) {
        return Err(format!(
            "driver {} needs the session command layer (unsupported here)",
            ir.driver
        ));
    }
    let device = load_spec_device(ir.driver)?;
    let seed = spec.run_seeds.first().copied().unwrap_or((100, 0));

    let state = Rc::new(RefCell::new(State::default()));
    let hook = IrHook {
        ir: *ir,
        state: state.clone(),
    };
    let engine_out = run_model_steps_with_hook(
        &root,
        &spec.models,
        seed,
        &spec.steps,
        &device,
        Some(Box::new(hook)),
    )
    .map_err(|e| format!("engine error: {e}"))?;

    let actual = match ir.output {
        OutputMode::Project => state.borrow().render(ir),
        OutputMode::Segment => normalize(&engine_out),
    };
    let ref_path = refs_dir().join(format!("{case}.ref"));
    let expected = normalize(
        &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
    );
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{case}: ref mismatch vs {}\n{}",
            ref_path.display(),
            first_diff(&expected, &actual)
        ))
    }
}
