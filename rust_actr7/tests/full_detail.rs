//! Full-detail differential traces — project-authored oracles.
//!
//! The regression corpus mostly uses the trace detail its upstream stub chose.
//! These cases rerun a small model with maximal narration (`:trace-detail
//! high`, `:act high`, `:pct t`, `:cst t`, …) and compare byte-exact against
//! SBCL output captured by `fixtures/full-detail/regen-full.sh`.
//!
//! The point is to expose numeric/event-ordering drift that the ordinary
//! refs suppress. Do not regenerate these from the Rust engine.

use std::fs;
use std::path::PathBuf;

use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::{run_model_steps, Step};

mod common;
use common::{
    crate_dir, first_diff, fixture_model, load_device, meta_value, normalize_eol, parse_seed_str,
};

macro_rules! full_case {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

full_case!(noisy_count_full, "noisy-count-full");
full_case!(bll_count_full, "bll-count-full");
full_case!(mp_test_full, "mp-test-full");
full_case!(ul_test_full, "ul-test-full");
full_case!(attend_full, "attend-full");

fn traces_dir() -> PathBuf {
    crate_dir()
        .join("fixtures")
        .join("full-detail")
        .join("traces")
}

fn run_case(name: &str) -> Result<(), String> {
    let trace_path = traces_dir().join(format!("{name}.trace"));
    let expected =
        fs::read_to_string(&trace_path).map_err(|e| format!("{}: {e}", trace_path.display()))?;
    let meta_path = trace_path.with_extension("meta");
    let meta =
        fs::read_to_string(&meta_path).map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let model = fixture_model(&meta_value(&meta, "model")?);
    let seed = parse_seed_str(&meta_value(&meta, "seed")?)?;
    let run_time: f64 = meta_value(&meta, "run")?
        .parse()
        .map_err(|_| format!("{name}: bad run time"))?;

    let mut steps = Vec::new();
    let settings = meta_value(&meta, "settings")?;
    if !settings.eq_ignore_ascii_case("nil") {
        let forms =
            parse_all(&settings).map_err(|e| format!("{name}: bad settings {settings:?}: {e}"))?;
        let Sexp::List(items) = forms
            .first()
            .ok_or_else(|| format!("{name}: empty settings"))?
        else {
            return Err(format!("{name}: settings is not a list: {settings}"));
        };
        for pair in items.chunks(2) {
            let [key, value] = pair else {
                return Err(format!("{name}: odd settings plist: {settings}"));
            };
            let key = key
                .as_sym()
                .ok_or_else(|| format!("{name}: settings key is not a symbol: {key:?}"))?;
            steps.push(Step::Sgp(key.to_string(), value.clone()));
        }
    }
    steps.push(Step::Run {
        secs: run_time,
        trace: true,
        width: 22,
    });

    let device = load_device(&trace_path)?;
    let actual = run_model_steps(&[model], seed, &steps, &device)
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
