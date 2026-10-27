//! `actr_engine` — Rust port of the ACT-R 7 single-threaded kernel.
//!
//! The engine reproduces ACT-R's execution traces byte-exact against two
//! oracles (see `fixtures/README.md`):
//!
//! * `fixtures/regression/` — ACT-R's own `test-models` corpus (`.ref`),
//!   compared with ACT-R's `compare-test-model-files` comparator
//!   (`tests/test_models.rs`).
//! * `fixtures/extra-traces/` — project-authored scenarios, byte-exact
//!   (`tests/extra_traces.rs`).
//!
//! Lisp-side quirks in `PORTING-NOTES.md`.
//!
//! Contract (mirrors `fixtures/run-traces.lisp`):
//! 1. Read the model file at `<actr_root>/<subpath with ';' as separator>`.
//! 2. Seed the RNG with `seed` (equivalent of `(sgp :seed (S C))`).
//! 3. Reset and run the model for `run_time` seconds of model time.
//! 4. Return the trace bytes that Lisp would write via `(sgp :v ...)`:
//!    lines from reset through end of run, byte-exact, including the
//!    `------ Stopped because ...` terminator. No banner, no warnings.

pub mod engine;
/// The native Environment GUI (`actr env`), behind the `env` feature so the
/// core engine and the trace test suites stay dependency-free.
#[cfg(feature = "env")]
pub mod env;
pub mod json;
pub mod model;
pub mod rand;
pub mod rpc;
pub mod sexpr;
pub mod trace;
/// WebAssembly entry points (compiled only for `wasm32-unknown-unknown`).
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use engine::{DeviceSetup, DeviceText, EngineError, EventHook, EventView, PreResult, Verdict};
pub use model::{load_models, ChunkDesc, Model};

use std::collections::HashMap;
use std::path::Path;

/// Run one model and return its trace as exact bytes.
///
/// * `actr_root` — filesystem path of the `actr7.x` checkout.
/// * `model` — model location in manifest form, e.g.
///   `"tutorial;unit1;count.lisp"` (`;` separates directories, as in the
///   ACT-R logical host `ACT-R:tutorial;unit1;count.lisp`). Task setup
///   files (which load a model plus build device state) work too.
/// * `seed` — `(sgp :seed ...)` value: initial seed + advance count.
///   Applied to the Lisp-identical MT stream ([`rand::Mt`]); draws happen
///   for activation noise (`:ans`), utility noise (`:egs`), and `:er`
///   tie-breaking, in source order.
/// * `run_time` — seconds of model time passed to `(run ...)`; the last
///   defined model in the file runs (matches `define-model` currency).
/// * `device` — scripted device input (window texts with appearance
///   times), mirroring what a task file builds with `open-exp-window` /
///   `add-text-to-exp-window` after install.
pub fn run_model(
    actr_root: &Path,
    model: &str,
    seed: (u64, u64),
    run_time: f64,
    device: &DeviceSetup,
) -> Result<String, EngineError> {
    run_model_with(
        actr_root,
        model,
        seed,
        run_time,
        device,
        trace::MODULE_WIDTH,
    )
}

/// Like [`run_model`], but with an explicit trace module-column width.
/// ACT-R derives this from `max-module-name-length`, which test-model stubs
/// may override (e.g. `tutorial-unit1-count-trace` pins 13); the spec corpus
/// captures the value per run.
pub fn run_model_with(
    actr_root: &Path,
    model: &str,
    seed: (u64, u64),
    run_time: f64,
    device: &DeviceSetup,
    module_width: usize,
) -> Result<String, EngineError> {
    let rel = model.replace(';', std::path::MAIN_SEPARATOR_STR);
    let path = actr_root.join(rel);
    let models = load_models(&path)?;
    let def = models
        .last()
        .ok_or_else(|| EngineError::Model(format!("no define-model in {model}")))?;
    engine::Engine::new_with_width(def.clone(), seed, run_time, device.clone(), module_width)?.run()
}

/// Run a model whose Lisp source is already in memory, returning its trace as
/// exact bytes. The filesystem-free twin of [`run_model`]: `source` is the
/// whole model file (the last `define-model` in it runs), and `seed`,
/// `run_time` and `device` have the same meaning. This is the entry point for
/// embedders with no ACT-R checkout on disk — e.g. the wasm build ([`wasm`]).
pub fn run_from_source(
    source: &str,
    seed: (u64, u64),
    run_time: f64,
    device: &DeviceSetup,
) -> Result<String, EngineError> {
    run_from_source_with(source, seed, run_time, device, trace::MODULE_WIDTH)
}

/// Like [`run_from_source`], but with an explicit trace module-column width.
pub fn run_from_source_with(
    source: &str,
    seed: (u64, u64),
    run_time: f64,
    device: &DeviceSetup,
    module_width: usize,
) -> Result<String, EngineError> {
    let models = model::load_models_from_str(source)?;
    let def = models
        .last()
        .ok_or_else(|| EngineError::Model("no define-model in source".to_string()))?;
    engine::Engine::new_with_width(def.clone(), seed, run_time, device.clone(), module_width)?.run()
}

/// One driver step from a per-case test file (or a recovered driver spec).
#[derive(Debug, Clone)]
pub enum Step {
    /// `(run secs)` with the trace state and module width in effect.
    Run {
        secs: f64,
        trace: bool,
        width: usize,
    },
    /// `(run-full-time secs)` — like `Run`, but runs to the time limit. The
    /// recorded `seed` is the RNG state ACT-R had at the run's start (the
    /// task's setup draws between runs are not otherwise replayed), so it is
    /// re-applied to keep the stream aligned.
    RunFullTime {
        secs: f64,
        trace: bool,
        width: usize,
        seed: Option<(u64, u64)>,
    },
    /// `(run-until-time end)` (`scheduling.lisp:1030`): run to an absolute
    /// model time (seconds).
    RunUntilTime {
        time: f64,
        trace: bool,
        width: usize,
    },
    /// `(run-n-events n)` (`scheduling.lisp:1081`): run until `n` events have
    /// been processed.
    RunNEvents {
        count: u64,
        trace: bool,
        width: usize,
    },
    /// `(model-output "…")`: a literal echoed line (the tutorial stubs echo
    /// the driver call before running).
    ModelOutput(String),
    /// `(goal-focus <spec>)`.
    GoalFocus(ChunkDesc),
    /// `(sgp :seed (S C))`.
    Seed((u64, u64)),
    /// Consume `n` draws from the random stream (the task's setup draws
    /// between runs, which the engine does not otherwise execute).
    ConsumeRng(u64),
    /// `(sdm <spec>...)`: print the declarative-memory chunks matching the
    /// task's spec (`commands/dm-commands.lisp:491`).
    Sdm(Vec<String>),
    /// `(no-output (sdm <spec>...))` followed by `(model-output "~s" names)`:
    /// suppress the chunk dump but print the returned name list.
    SdmNames(Vec<String>),
    /// `(record-history "goal" "visual" …)`: the buffers the BOLD predictor
    /// traces (the demand recorder).
    RecordHistory(Vec<String>),
    /// `(predict-bold-response)`: print the BOLD demand table.
    PredictBold,
    /// `(sgp :trace-detail X)`.
    SetDetail(trace::Detail),
    /// `(sgp :key value)` for any other parameter (applied at run time).
    Sgp(String, sexpr::Sexp),
    /// `(reset)`.
    Reset,
    /// `(pdisable <name>...)`: skip the named productions.
    PDisable(Vec<String>),
    /// `(penable <name>...)`: re-enable the named productions.
    PEnable(Vec<String>),
    /// `(pbreak <name>...)` without printing the instantiation (the task
    /// wraps the run in `no-output`).
    PBreak(Vec<String>),
    /// `(buffer-chunk <name>...)`: the task driver's final buffer dump.
    BufferChunk(Vec<String>),
    /// `(printed-buffer-chunk <name>...)`: the task driver's returned
    /// multi-buffer dump (chunk lines have no blank between buffers).
    PrintedBufferChunk(Vec<String>),
    /// Schedule a no-op event at absolute model time (real-time slack hook).
    DummyEvent(i64),
    /// Emit a raw task warning line.
    EmitWarning(String),
    /// `(start-hand-at-touch <hand>)`.
    StartHandAtTouch(String),
    /// `(mod-focus slot value ...)`.
    ModFocus(Vec<(String, sexpr::Sexp)>),
    /// `(add-visicon-features (slot value) ...)`.
    AddVisicon(Vec<(String, sexpr::Sexp)>),
    /// tracker task setup: add the prompt feature.
    TrackerSetup(Vec<(String, sexpr::Sexp)>),
    /// `zz-extended-motor-actions` task driver (`run-test`/`testN`).
    ZzDriver,
    /// `associative-learning` task driver (`run-it`/`approach-1-results`).
    AlDriver,
    /// Start collecting the per-run response times/answers for
    /// `fan-no-pm`'s `fan-experiment`.
    FanCollect,
    /// Print `fan-no-pm`'s `output-person-location` table from the runs
    /// collected since [`Step::FanCollect`].
    FanSummary,
    /// Start collecting paired-associate prompt trials from the spec runs.
    PairedCollect,
    /// Print the paired-associate latency/accuracy summary.
    PairedSummary,
    /// `siegler-experiment` task driver: fixed per-trial seeds, digit sounds,
    /// and the response matrix/statistics table. `fact_base` applies the
    /// `f00`..`f40` base levels; `all_base` applies `set-all-base-levels`.
    SieglerDriver {
        first_seed: u64,
        sets: u32,
        rt: Option<f32>,
        mp: Option<f32>,
        ans: Option<f32>,
        esc: Option<bool>,
        fact_base: Option<f32>,
        all_base: Option<f32>,
    },
    /// `(modify-visicon-features ...)` at an absolute time.
    ModifyVisicon {
        at_ms: i64,
        name: String,
        x: i64,
        y: i64,
    },
    /// A driver-level command call the extractor recorded generically
    /// (`(:command NAME ARG…)`), e.g. `pp`, `whynot`, `all-productions`,
    /// `mp-queue-text`. Replayed through the RPC command layer
    /// ([`rpc::session::Session::evaluate`]) so the spec exercises the same
    /// dispatch a Python client hits; `run_model_steps` rejects it (the
    /// session driver owns it).
    Command {
        name: String,
        args: Vec<sexpr::Sexp>,
    },
}

/// Run a scripted sequence of steps over one or more model files (the
/// test-model driver form: repeated `run` bounded relative to the current
/// model time, `goal-focus` between runs). Returns the concatenated trace.
///
/// The RNG is seeded once from `seed` (the first run's effective seed); Lisp
/// `(run ...)` does not reseed, and none of the current PURE/MULTI specs
/// change `:seed` mid-driver.
pub fn run_model_steps(
    actr_root: &Path,
    models: &[String],
    seed: (u64, u64),
    steps: &[Step],
    device: &DeviceSetup,
) -> Result<String, EngineError> {
    run_model_steps_with_hook(actr_root, models, seed, steps, device, None)
}

/// Like [`run_model_steps`], but installs an [`EventHook`] on the engine
/// before the driver steps run (the hook is what the `:SCOPE :HOOK` cases use
/// to reproduce ACT-R's `add-pre/post-event-hook` filters).
pub fn run_model_steps_with_hook(
    actr_root: &Path,
    models: &[String],
    seed: (u64, u64),
    steps: &[Step],
    device: &DeviceSetup,
    hook: Option<Box<dyn EventHook>>,
) -> Result<String, EngineError> {
    let mut defs = Vec::new();
    let mut user_fns: HashMap<String, model::UserDefun> = HashMap::new();
    let mut user_vars = HashMap::new();
    let mut commands: HashMap<String, String> = HashMap::new();
    for m in models {
        let rel = m.replace(';', std::path::MAIN_SEPARATOR_STR);
        let path = actr_root.join(rel);
        // Test-model stubs load a task file (defuns, experiment code) that
        // in turn loads the actual model. Only files that define a model are
        // fed to the strict loader.
        if path.is_file() {
            if file_defines_model(&path) {
                defs.extend(load_models(&path)?);
            }
            // Task files (also listed in the spec's model chain) carry the
            // `defun`s and `add-act-r-command` registrations.
            if let Ok(src) = std::fs::read_to_string(&path) {
                user_fns.extend(model::parse_defuns(&src));
                user_vars.extend(model::parse_task_globals(&src));
                commands.extend(model::parse_commands(&src));
            }
        }
        // A `models/X.lisp` model commonly has a sibling task file
        // `X.lisp` defining helper `defun`s (e.g. `single-bind`).
        if let Some(task) = companion_task_file(&path) {
            if let Ok(src) = std::fs::read_to_string(&task) {
                user_fns.extend(model::parse_defuns(&src));
                user_vars.extend(model::parse_task_globals(&src));
                commands.extend(model::parse_commands(&src));
            }
        }
    }
    // A scheduled command name resolves to its registered function.
    for (cmd, fname) in commands {
        if let Some(def) = user_fns.get(&fname).cloned() {
            user_fns.insert(cmd, def);
        }
    }
    let def = defs
        .last()
        .ok_or_else(|| EngineError::Model("no define-model in step script".to_string()))?;
    let mut eng = engine::Engine::new(def.clone(), seed, 0.0, device.clone())?;
    eng.set_event_hook(hook);
    eng.set_user_fns(user_fns);
    eng.set_user_vars(user_vars);
    eng.apply_model_commands()?;
    for step in steps {
        match step {
            Step::Run { secs, trace, width } => {
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_secs(*secs)?;
            }
            Step::RunFullTime {
                secs,
                trace,
                width,
                seed,
            } => {
                // `(100 0)` is the extractor's placeholder for a run seed it
                // could not match to a literal, so it is not the RNG state
                // ACT-R was in — applying it would jump the stream.
                if let Some(s) = seed {
                    if *s != (100, 0) {
                        eng.set_seed(*s);
                    }
                }
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_full_time(*secs)?;
            }
            Step::RunUntilTime { time, trace, width } => {
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_until(*time)?;
            }
            Step::RunNEvents {
                count,
                trace,
                width,
            } => {
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_n_events(*count)?;
            }
            Step::ModelOutput(s) => eng.model_output(s),
            Step::GoalFocus(desc) => eng.goal_focus(desc)?,
            Step::Seed(s) => eng.set_seed(*s),
            Step::ConsumeRng(n) => eng.consume_rng(*n),
            Step::Sdm(spec) => {
                eng.print_sdm(spec);
            }
            Step::SdmNames(spec) => {
                let mark = eng.out_len();
                let names = eng.print_sdm(spec);
                eng.truncate_out(mark);
                eng.model_output(&format!("({})", names.join(" ")));
            }
            Step::RecordHistory(buffers) => eng.set_bold_buffers(buffers),
            Step::PredictBold => eng.print_bold(),
            Step::SetDetail(d) => eng.set_detail(*d),
            Step::Sgp(k, v) => eng.apply_sgp(k, v)?,
            Step::Reset => eng.reset()?,
            Step::PDisable(names) => eng.pdisable(names),
            Step::PEnable(names) => eng.penable(names),
            Step::PBreak(names) => eng.pbreak_quiet(names),
            Step::BufferChunk(buffers) => {
                eng.print_buffer_chunk(buffers);
            }
            Step::PrintedBufferChunk(buffers) => {
                eng.print_printed_buffer_chunk(buffers);
            }
            Step::DummyEvent(at_ms) => eng.schedule_dummy(*at_ms),
            Step::EmitWarning(msg) => eng.emit_warning(msg),
            Step::StartHandAtTouch(hand) => eng.start_hand_at_touch(hand),
            Step::ModFocus(slots) => eng.mod_focus(slots),
            Step::AddVisicon(slots) => {
                eng.add_visicon(slots);
            }
            Step::TrackerSetup(slots) => eng.tracker_setup(slots),
            Step::ZzDriver => eng.zz_driver()?,
            Step::AlDriver => eng.al_driver()?,
            Step::FanCollect => eng.fan_collect_start(),
            Step::FanSummary => eng.fan_no_pm_summary(),
            Step::PairedCollect => eng.paired_collect_start(),
            Step::PairedSummary => eng.paired_summary(),
            Step::SieglerDriver {
                first_seed,
                sets,
                rt,
                mp,
                ans,
                esc,
                fact_base,
                all_base,
            } => eng.siegler_driver(
                *first_seed,
                *sets,
                *rt,
                *mp,
                *ans,
                *esc,
                *fact_base,
                *all_base,
            )?,
            Step::ModifyVisicon { at_ms, name, x, y } => {
                eng.schedule_modify_visicon(*at_ms, name, *x, *y)
            }
            // Command steps are replayed through the RPC command layer, which
            // this engine-level driver has no access to; the spec harness uses
            // its session driver (`tests/test_models.rs::run_spec_session`)
            // for any spec containing one.
            Step::Command { name, .. } => {
                return Err(EngineError::Model(format!(
                    "command step {name:?} requires the session driver"
                )));
            }
        }
    }
    Ok(eng.output())
}

/// A `.../models/X.lisp` model's sibling task file `.../X.lisp`, if present
/// (test-model convention: the task file defines helper `defun`s and loads
/// the model).
fn companion_task_file(model_path: &Path) -> Option<std::path::PathBuf> {
    let parent = model_path.parent()?;
    if parent.file_name()?.to_str()? != "models" {
        return None;
    }
    let candidate = parent.parent()?.join(model_path.file_name()?);
    candidate.is_file().then_some(candidate)
}

/// Whether a file contains a top-level `define-model` (possibly `-fct`)
/// form. Used to skip test-model task files, which strict `load_models`
/// would reject on their `defun`/experiment forms.
pub fn file_defines_model(path: &Path) -> bool {
    let Ok(src) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(forms) = sexpr::parse_all(&src) else {
        return false;
    };
    forms.iter().any(|f| match f {
        sexpr::Sexp::List(v) => matches!(
            v.first(),
            Some(sexpr::Sexp::Sym(s)) if s == "DEFINE-MODEL" || s == "DEFINE-MODEL-FCT"
        ),
        _ => false,
    })
}
