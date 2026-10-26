//! Event-hook (`.lisp` `add-pre/post-event-hook`) regression cases.
//!
//! ACT-R's `:SCOPE :HOOK` stubs run the model through an arbitrary-Lisp
//! filter. Each case here writes that filter directly as an [`EventHook`]:
//!
//! * [`HookCase::check_segment`] compares the engine's raw trace. The hook
//!   may toggle `:v` and/or break the run, so the trace is the filtered
//!   output (the `pcomp-issues` and trace-window families).
//! * [`HookCase::check_projection`] compares the lines a hook collected in a
//!   [`Projection`] (the demo2 `format-event`/`model-output` family).
//!
//! Every case also owns its driver (models/seed/steps/device) inline. The
//! expected output is the case's own `.ref`.
//!
//! See `PORTING-NOTES.md` #269 and `fixtures/regression/README.md`.

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

use actr_engine::json::Json;
use actr_engine::rpc::session::Session;
use actr_engine::{run_model_steps_with_hook, EventHook, EventView, PreResult, Step};

use super::{
    apply_session_step, first_diff, load_spec_device, models_dir, normalize, refs_dir, NoRemote,
};

// --------------------------------------------------------------- predicates

pub fn action(ev: &EventView, name: &str) -> bool {
    ev.action.eq_ignore_ascii_case(name)
}

pub fn module(ev: &EventView, name: &str) -> bool {
    ev.module.eq_ignore_ascii_case(name)
}

pub fn buffer(ev: &EventView, name: &str) -> bool {
    ev.buffer
        .as_deref()
        .is_some_and(|b| b.eq_ignore_ascii_case(name))
}

pub fn production(ev: &EventView, name: &str) -> bool {
    ev.production
        .as_deref()
        .is_some_and(|p| p.eq_ignore_ascii_case(name))
}

/// The fan-sentence trace-window filter: suppress the trace until the second
/// retrieval starts at 585 ms, then stop after the retrieval's
/// conflict-resolution.
#[derive(Default)]
pub struct ActTraceWindow {
    enabled: bool,
    stop_after: bool,
    conflicts: u32,
}

impl EventHook for ActTraceWindow {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if self.stop_after {
            self.enabled = false;
            self.stop_after = false;
        }
        if ev.time_ms >= 585 && action(ev, "start-retrieval") {
            self.enabled = true;
        }
        if self.enabled && ev.time_ms > 585 && action(ev, "conflict-resolution") {
            self.conflicts += 1;
            if self.conflicts == 1 {
                self.stop_after = true;
            }
        }
        PreResult::trace(self.enabled)
    }
}

// -------------------------------------------------------------- projection

/// Lines collected by a projection hook. `record` mirrors the demo2 filters'
/// `push-last`/`...` shape; `finish` applies `butlast` and the stub's
/// `model-output` separator.
#[derive(Default)]
pub struct Projection {
    lines: Vec<String>,
    started: bool,
    last_dot: bool,
}

impl Projection {
    pub fn record(&mut self, line: &str, keep: bool, else_dots: bool) {
        if keep {
            self.push_line(line);
        } else if else_dots && self.started && !self.last_dot {
            self.push_ellipsis();
        }
    }

    /// Append an already-selected `format-event` line. Used by filters whose
    /// keep/dot state machine is not the simple `record` shape.
    pub fn push_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
        self.started = true;
        self.last_dot = false;
    }

    /// Append `"..."` unless the previous line already is one.
    pub fn push_ellipsis(&mut self) {
        if !self.last_dot {
            self.lines.push("...".to_string());
            self.last_dot = true;
        }
    }

    pub fn finish(&self, join: &str, butlast: bool) -> Vec<String> {
        let mut lines = self.lines.clone();
        if butlast {
            lines.pop();
        }
        normalize(&lines.join(join))
    }
}

// ------------------------------------------------------------------- case

/// One hook case: an inline driver (models/seed/steps/device) plus a direct
/// [`EventHook`]. `device` defaults to the case name; pass [`Self::device`]
/// when the case shares another case's device sidecar (the demo2 family).
pub struct HookCase {
    case: String,
    models: Vec<String>,
    seed: (u64, u64),
    steps: Vec<Step>,
    device: Option<String>,
}

impl HookCase {
    pub fn new(case: &str) -> Self {
        Self {
            case: case.to_string(),
            models: Vec::new(),
            seed: (100, 0),
            steps: Vec::new(),
            device: None,
        }
    }

    pub fn models(mut self, models: &[&str]) -> Self {
        self.models = models.iter().map(|m| m.to_string()).collect();
        self
    }

    pub fn seed(mut self, seed: (u64, u64)) -> Self {
        self.seed = seed;
        self
    }

    pub fn steps(mut self, steps: Vec<Step>) -> Self {
        self.steps = steps;
        self
    }

    pub fn device(mut self, name: &str) -> Self {
        self.device = Some(name.to_string());
        self
    }

    /// Run the driver with `hook`, then compare the engine's raw trace to the
    /// case's `.ref` (the hook's `:v`/break effects are what make it match).
    pub fn check_segment(self, hook: impl EventHook + 'static) -> Result<(), String> {
        let case = self.case.clone();
        let actual = normalize(&self.run(Box::new(hook))?);
        compare(&case, actual)
    }

    /// Run the driver with `hook`, then compare the hook's projected lines to
    /// the case's `.ref`.
    pub fn check_projection(
        self,
        hook: impl EventHook + 'static,
        lines: Rc<RefCell<Projection>>,
        join: &str,
        butlast: bool,
    ) -> Result<(), String> {
        let case = self.case.clone();
        self.run(Box::new(hook))?;
        let actual = lines.borrow().finish(join, butlast);
        compare(&case, actual)
    }

    fn run(self, hook: Box<dyn EventHook>) -> Result<String, String> {
        let root = models_dir();
        if !root.is_dir() {
            return Err(format!(
                "vendored model fixtures not found at {}",
                root.display()
            ));
        }
        let HookCase {
            case,
            models,
            seed,
            steps,
            device,
        } = self;
        let device_name = device.as_deref().unwrap_or(&case);
        let device = load_spec_device(device_name)?;
        let needs_session = steps.iter().any(|s| match s {
            Step::Command { .. } => true,
            Step::Sgp(k, _) => k.eq_ignore_ascii_case(":STARTING-PARAMETERS"),
            _ => false,
        });
        if needs_session {
            run_session_hook(&root, &models, seed, &steps, &device, hook)
        } else {
            run_model_steps_with_hook(&root, &models, seed, &steps, &device, Some(hook))
                .map_err(|e| format!("{case}: engine error: {e}"))
        }
    }
}

fn compare(case: &str, actual: Vec<String>) -> Result<(), String> {
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

/// Drive a command-bearing or `:STARTING-PARAMETERS` driver through the RPC
/// [`Session`], installing the hook on the engine before the steps run.
fn run_session_hook(
    root: &std::path::Path,
    models: &[String],
    seed: (u64, u64),
    steps: &[Step],
    device: &actr_engine::DeviceSetup,
    hook: Box<dyn EventHook>,
) -> Result<String, String> {
    let mut session = Session::new(root.to_path_buf(), seed);
    let mut remote = NoRemote;
    for m in models {
        session
            .evaluate(
                "load-act-r-model",
                None,
                &[Json::Str(m.clone())],
                &mut remote,
            )
            .map_err(|e| format!("load {m}: {e}"))?;
    }
    session.set_device(device.clone())?;
    session
        .engine_mut()
        .ok_or("no model loaded")?
        .set_event_hook(Some(hook));
    for step in steps {
        apply_session_step(&mut session, step, &mut remote)?;
    }
    Ok(session.take_trace_output())
}
