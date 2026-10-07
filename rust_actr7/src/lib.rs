//! `actr_engine` — Rust port of the ACT-R 7 single-threaded kernel.
//!
//! Status:
//! - M1: golden harness (`tests/golden.rs`) — green.
//! - M2a (done): S-expression reader ([`sexpr`]) + `define-model` loader
//!   ([`model`]).
//! - M2b (done): discrete-event [`engine`] — scheduler, buffers,
//!   procedural match/select/fire, exact-match declarative retrieval.
//!   First 3 M0 goldens reproduce byte-exact.
//! - M2c (done): Lisp-identical MT stream ([`rand`]), activation noise
//!   (`:ans`), utility noise (`:egs`), `:er` ties. Two synthetic noisy
//!   goldens reproduce byte-exact, including failures and non-50 ms
//!   latencies. Still out: `:bll`/`:mas`/`:mp` (activation math),
//!   `:ul` (utility learning), imaginal/temporal/devices.
//! - M4 (done): base-level learning — reference histories via
//!   merge-on-clear, decay equation wired into retrieval, `bll-count`
//!   golden byte-exact. Remaining subsymbolics: `:mas` spreading,
//!   `:mp` partial matching, `:ul`.
//! - M5a (done): device buffers — scripted window device, visicon +
//!   onset stuffing, visual-location find, move-attention + encoding,
//!   manual press-key pipeline (prep/init/output/finish), implicit
//!   `?buffer> state free` queries. `attend` golden byte-exact.
//! - M5b (done): imaginal buffer, `+visual> clear`, digit keys +
//!   prep differences, VL SET-copy naming. `paired-static` golden
//!   (failed recall → study → learned recall → keypress → idle stop)
//!   byte-exact.
//! - M6 (done): multi-trial paired (`paired-2x2`, 2 pairs × 2 blocks,
//!   fixed 60 s schedule): harvest-named pair chunks, hand-change prep
//!   differences, long-run RNG/learning stability. Byte-exact.
//! - M7 (done): full paired task (`paired-full`, 2 pairs, 5 s prompt +
//!   study segments with window clears, single 20 s run): CLEAR/END
//!   sidecar actions, maintenance event classes, time-limit tail.
//!   Byte-exact.
//! - M8 (done): partial matching — signature candidacy, similarity
//!   scoring with var-skipping, `mp-test` golden (similarity decides
//!   between two non-matching candidates). Byte-exact.
//! - M9 (done): utility learning — selection history, reward
//!   propagation with recency discount, `ul-test` penalty-flip golden.
//!   Byte-exact (`:iu` honored; `:ult` trace lines not implemented).
//! - M10 (in progress): ACT-R's own regression corpus
//!   (`actr7.x/test-models/`) is integrated as a second oracle:
//!   `golden/regen-specs.sh` extracts runner specs, `tests/test_models.rs`
//!   diffs engine output against `.ref` with ACT-R's comparator, and
//!   `golden/test-models/enabled.txt` is the green allowlist (38 specs so
//!   far). This milestone added per-spec trace module width
//!   (`max-module-name-length` overrides), integer-vs-float slot printing,
//!   and a scripted driver (`run_model_steps`: repeated `run`, `goal-focus`,
//!   `:v`/`:trace-detail`/`:seed`, `(reset)`), which made the multi-run
//!   `semantic` trace green. Added `define-chunks`, model-body
//!   `declare-buffer-usage`/`spp`/`set-base-levels`/`set-similarities`,
//!   `?buffer> buffer empty|full`, `+goal>` requests, `-` request slots, and
//!   spreading (`:mas`, with DM fan tables) and signature-based
//!   partial-matching candidacy — the unit5 modeling cluster now runs
//!   (5 specs green), and `:act` activation traces plus request-spec
//!   `=buffer` chunk copies made `sji-merge` green; `:crt`
//!   conflict-resolution text and unnamed `add-dm` auto-naming made
//!   `buffer-queries` green; and LHS variable slot names, `slot =var`
//!   requiring filled, named-merge renaming, and type-free `isa` matching
//!   made `semantic-dynamic` green. Remaining: `!bind!`/`!eval!`,
//!   `require-extra`, `schedule-event` scaffolding, and multi-model traces.
//!   Chunk-valued references + merge aliasing and `!stop!` made
//!   `merging-test` green; the reference-count base-level fix, retrieval
//!   finsts (`:recently-retrieved`), and symbol similarities made the
//!   unit5 `v4-p3-run`/`v6-run-p3` traces green. `require-extra` is now
//!   recognized/recorded and chunk-type default slots are applied, and the
//!   `blending` extra is ported: first-class inequality request constraints
//!   (`> < >= <=`), the `BLENDING` buffer/module with its `state free/busy`
//!   and `error nil` queries, and `start-blending` (signature/inequality
//!   candidacy in DM-alist order, softmax probabilities at temperature
//!   `:tmp`/`√2·:ans`, numeric weighted averages, similarity-based chunk
//!   values, log-sum-exp blended activation, and the `:blt` trace) — making
//!   `blending` green. The `:cst` conflict-set trace (instantiated
//!   `print-production-text` plus production parameters) made
//!   `tutorial-unit8-simple-ppm-model-initial-run` green. Production
//!   compilation (`:ppm` procedural partial matching, the
//!   `goal-focus-fct`/`(car (define-chunks …))` model form, and the
//!   `:epl`/`:pct` composition/trace for the goal test+modify case) made
//!   `ppm-compilation` green. Runtime `:sgp` driver steps plus partial-match
//!   annotations/utility in the `:cst` trace made the `simple-ppm` family
//!   green (`simple-ppm-test`, `…-ppm-run`, `…-medium-chosen-run`).
//!   `*buffer>` modification requests and indirect-action validity
//!   (`:cia`) cover run 1 of `indirect-action-compilation`; indirect-action
//!   composition + installing compiled productions into the runtime made
//!   `indirect-action-compilation` green. A bounded `!eval!`/`!bind!`
//!   evaluator (pure operators + `safe-` variants; unknown heads error)
//!   made `dynamic-slot-compilation` green. A bounded task-file `defun`
//!   interpreter plus the sole-`isa` matching rule made `bind-test` and
//!   `remote-bind-test` green. A `:ACT` `:sgp` arm enabled the unit5-v3
//!   activation trace (`tutorial-unit5modeling-v3-first-run`). A scripted
//!   device window for the test-model corpus (an `ADD-TEXT` sidecar,
//!   `:attended`/`nil` request-parameter matching, the QWERTY key table,
//!   and a `clear_on_keypress` behavior that re-encodes the vanished
//!   attended object) made `demo2` green. Model-body `schedule-event`
//!   plus a bounded task-command dispatch (`add-act-r-command` aliases,
//!   `goal-focus`/`set-buffer-chunk`/`schedule-event-relative`/`act-r-random`),
//!   `declare-buffer-usage`, `(spp :reward R)` propagation and its `:ult`
//!   update trace made the `ul-issues` cluster green (fixing the extractor's
//!   stray runtime `:goal-focus` steps and a `run_secs` event-drop bug).
//!   The `TEMPORAL` module (tick/noise dynamics, `isa time`/`isa clear`
//!   requests, `:record-ticks`, `delete-event` via heap retain) and
//!   `(spp :at S)` action-time overrides made `test-temporal` green.
//!   `:randomize-time` (`:vpft`/`:vidt`) jitter for production, vision,
//!   imaginal and motor latencies made `demo2-randomized` green.
//!   The full `:cia`/`:rir` production-compilation engine (composed
//!   productions instantiate indirect-action variables, expand indirect
//!   requests to chunk specs, retain indirect retrievals under `:rir`, and
//!   handle `!bind!` composition conditions) made
//!   `retain-indirect-retrievals` green across all three runs. The
//!   partial-matching `:full` activation trace, finst removal trace,
//!   retrieval-threshold messages, `buffer empty` vs failure, and
//!   `!eval!` task-command dispatch made `grouped` green.
//!   Remaining: task-supplied commands
//!   (`grouped`), production validation (`test-bad-productions`, blocked
//!   by cross-test warning state), `!eval!` timing (`dynamic-real-time`),
//!   other `require-extra` behaviors (associative-learning/act-touch/
//!   z-threads/extended-motor), `schedule-event` scaffolding, and
//!   multi-model traces.
//! - Next: audio/speech devices, production compilation.
//!
//! Contract (mirrors `golden/run-golden.lisp`):
//! 1. Read the model file at `<actr_root>/<subpath with ';' as separator>`.
//! 2. Seed the RNG with `seed` (equivalent of `(sgp :seed (S C))`).
//! 3. Reset and run the model for `run_time` seconds of model time.
//! 4. Return the trace bytes that Lisp would write via `(sgp :v ...)`:
//!    lines from reset through end of run, byte-exact, including the
//!    `------ Stopped because ...` terminator. No banner, no warnings.

pub mod engine;
pub mod model;
pub mod rand;
pub mod sexpr;
pub mod trace;

pub use engine::{DeviceSetup, DeviceText, EngineError};
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
    run_model_with(actr_root, model, seed, run_time, device, trace::MODULE_WIDTH)
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

/// One driver step from a test spec (`golden/test-models/specs/*.spec`).
#[derive(Debug, Clone)]
pub enum Step {
    /// `(run secs)` with the trace state and module width in effect.
    Run {
        secs: f64,
        trace: bool,
        width: usize,
    },
    /// `(run-full-time secs)` — like `Run`, but runs to the time limit.
    RunFullTime {
        secs: f64,
        trace: bool,
        width: usize,
    },
    /// `(goal-focus <spec>)`.
    GoalFocus(ChunkDesc),
    /// `(sgp :seed (S C))`.
    Seed((u64, u64)),
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
    /// `(buffer-chunk <name>...)`: the task driver's final buffer dump.
    BufferChunk(Vec<String>),
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
    /// `(modify-visicon-features ...)` at an absolute time.
    ModifyVisicon {
        at_ms: i64,
        name: String,
        x: i64,
        y: i64,
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
    let mut defs = Vec::new();
    let mut user_fns: HashMap<String, model::UserDefun> = HashMap::new();
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
                commands.extend(model::parse_commands(&src));
            }
        }
        // A `models/X.lisp` model commonly has a sibling task file
        // `X.lisp` defining helper `defun`s (e.g. `single-bind`).
        if let Some(task) = companion_task_file(&path) {
            if let Ok(src) = std::fs::read_to_string(&task) {
                user_fns.extend(model::parse_defuns(&src));
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
    eng.set_user_fns(user_fns);
    for step in steps {
        match step {
            Step::Run { secs, trace, width } => {
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_secs(*secs)?;
            }
            Step::RunFullTime { secs, trace, width } => {
                eng.set_module_width(*width);
                eng.set_trace(*trace);
                eng.run_full_time(*secs)?;
            }
            Step::GoalFocus(desc) => eng.goal_focus(desc)?,
            Step::Seed(s) => eng.set_seed(*s),
            Step::SetDetail(d) => eng.set_detail(*d),
            Step::Sgp(k, v) => eng.apply_sgp(k, v)?,
            Step::Reset => eng.reset()?,
            Step::PDisable(names) => eng.pdisable(names),
            Step::PEnable(names) => eng.penable(names),
            Step::BufferChunk(buffers) => eng.print_buffer_chunk(buffers),
            Step::DummyEvent(at_ms) => eng.schedule_dummy(*at_ms),
            Step::EmitWarning(msg) => eng.emit_warning(msg),
            Step::StartHandAtTouch(hand) => eng.start_hand_at_touch(hand),
            Step::ModFocus(slots) => eng.mod_focus(slots),
            Step::AddVisicon(slots) => eng.add_visicon(slots),
            Step::TrackerSetup(slots) => eng.tracker_setup(slots),
            Step::ZzDriver => eng.zz_driver()?,
            Step::AlDriver => eng.al_driver()?,
            Step::ModifyVisicon {
                at_ms,
                name,
                x,
                y,
            } => eng.schedule_modify_visicon(*at_ms, name, *x, *y),
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Wiring check: the M0 goldens this crate must eventually reproduce
    /// are present. Green now and forever; the red test is `tests/golden.rs`.
    #[test]
    fn golden_fixtures_present() {
        for name in [
            "count",
            "semantic",
            "addition",
            "noisy-count",
            "noisy-count-7",
            "bll-count",
            "attend",
            "paired-static",
            "paired-2x2",
            "paired-full",
            "mp-test",
            "ul-test",
        ] {
            let trace = crate_dir()
                .join("golden")
                .join("traces")
                .join(format!("{name}.trace"));
            assert!(
                trace.is_file(),
                "missing golden fixture: {} (run golden/regen.sh)",
                trace.display()
            );
        }
    }
}
