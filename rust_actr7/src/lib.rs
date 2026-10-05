//! `actr_engine` — Rust port of the ACT-R 7 single-threaded kernel.
//!
//! M1: skeleton only. The golden test (`tests/golden.rs`) defines the
//! contract this crate must eventually satisfy; every engine entry point
//! below returns `EngineError::NotImplemented` until M2+ implements it.
//!
//! Contract (mirrors `golden/run-golden.lisp`):
//! 1. Read the model file at `<actr_root>/<subpath with ';' as separator>`.
//! 2. Seed the RNG with `seed` (equivalent of `(sgp :seed (S C))`).
//! 3. Reset and run the model for `run_time` seconds of model time.
//! 4. Return the trace bytes that Lisp would write via `(sgp :v ...)`:
//!    lines from reset through end of run, byte-exact, including the
//!    `------ Stopped because ...` terminator. No banner, no warnings.

use std::fmt;
use std::path::Path;

/// Errors from the (not yet implemented) engine.
#[derive(Debug)]
pub enum EngineError {
    /// M2+ has not implemented this yet. The string says what was attempted.
    NotImplemented(String),
    /// Filesystem failure reading models or goldens.
    Io(std::io::Error),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::NotImplemented(what) => write!(f, "not implemented: {what}"),
            EngineError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e)
    }
}

/// Run one model and return its trace as exact bytes.
///
/// * `actr_root` — filesystem path of the `actr7.x` checkout.
/// * `model` — model location in manifest form, e.g.
///   `"tutorial;unit1;count.lisp"` (`;` separates directories, as in the
///   ACT-R logical host `ACT-R:tutorial;unit1;count.lisp`).
/// * `seed` — `(sgp :seed ...)` value: initial seed + advance count.
/// * `run_time` — seconds of model time passed to `(run ...)`.
pub fn run_model(
    _actr_root: &Path,
    model: &str,
    _seed: (u64, u64),
    _run_time: f64,
) -> Result<String, EngineError> {
    Err(EngineError::NotImplemented(format!(
        "M2 has not implemented model execution yet (model: {model})"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Wiring check: the M0 goldens this crate must eventually reproduce
    /// are present. Green now and forever; the red test is `tests/golden.rs`.
    #[test]
    fn golden_fixtures_present() {
        for name in ["count", "semantic", "addition"] {
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
