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
//! - Next: audio/speech devices, `:ul` utility learning, production
//!   compilation.
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
pub use model::{load_models, Model};

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
    let rel = model.replace(';', std::path::MAIN_SEPARATOR_STR);
    let path = actr_root.join(rel);
    let models = load_models(&path)?;
    let def = models
        .last()
        .ok_or_else(|| EngineError::Model(format!("no define-model in {model}")))?;
    engine::Engine::new(def.clone(), seed, run_time, device.clone())?.run()
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
