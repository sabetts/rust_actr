//! Project-authored trace tests — **not** from ACT-R's regression suite.
//!
//! ACT-R ships no tests for these scenarios, so they are our own ground
//! truth: `fixtures/extra-traces/<name>.trace`, captured from the Lisp
//! system (`fixtures/regen.sh`) and compared **byte-exact** (CRLF -> LF
//! only), unlike the trim/case-insensitive comparator used for the ACT-R
//! refs in `test_models.rs`.
//!
//! The sibling `<name>.meta` supplies the model location, seed and run time;
//! an optional `<name>.device.sexp` scripts window input. These cases exist
//! because they exercise behavior the upstream corpus does not (activation
//! noise, base-level learning, device/window scaffolding).
//!
//! The cases are a static list of `#[test]`s, one per scenario.

use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::run_model;

mod common;
use common::{
    crate_dir, first_diff, load_device, meta_value, models_dir, normalize_eol, parse_seed_str,
};

macro_rules! trace_case {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

trace_case!(noisy_count, "noisy-count");
trace_case!(noisy_count_7, "noisy-count-7");
trace_case!(bll_count, "bll-count");
trace_case!(attend, "attend");
trace_case!(mp_test, "mp-test");
trace_case!(ul_test, "ul-test");

fn traces_dir() -> PathBuf {
    crate_dir().join("fixtures").join("extra-traces")
}

fn run_case(name: &str) -> Result<(), String> {
    let root = models_dir();
    if !root.is_dir() {
        return Err(format!(
            "vendored model fixtures not found at {}",
            root.display()
        ));
    }
    let trace_path = traces_dir().join(format!("{name}.trace"));
    let expected =
        fs::read_to_string(&trace_path).map_err(|e| format!("{}: {e}", trace_path.display()))?;
    let meta_path = trace_path.with_extension("meta");
    let meta =
        fs::read_to_string(&meta_path).map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let model = meta_value(&meta, "model")?;
    let seed = parse_seed_str(&meta_value(&meta, "seed")?)?;
    let run_time: f64 = meta_value(&meta, "run")?
        .parse()
        .map_err(|_| format!("{name}: bad run time"))?;

    let device = load_device(&trace_path)?;
    let actual = run_model(&root, &model, seed, run_time, &device)
        .map_err(|e| format!("{name}: engine error: {e}"))?;

    if normalize_eol(&actual) == normalize_eol(&expected) {
        Ok(())
    } else {
        let e: Vec<&str> = expected.lines().collect();
        let a: Vec<&str> = actual.lines().collect();
        Err(format!(
            "{name}: trace mismatch vs {}\n{}",
            trace_path.display(),
            first_diff(&e, &a)
        ))
    }
}
