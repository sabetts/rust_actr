//! Draw-sequence differential tests.
//!
//! `fixtures/draws/*.draws.sexp` is captured from SBCL by wrapping
//! `act-r-random` and `act-r-noise` (`regen-draws.sh`). The engine records the
//! same two entry points in `Mt::start_log`. Comparing the sequences catches
//! draw-order/count drift that never changes a printed trace.

use std::fs;
use std::path::{Path, PathBuf};

use actr_engine::engine::Engine;
use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::{load_models, DeviceSetup};

mod common;
use common::{crate_dir, meta_value, models_dir, parse_seed_str};

macro_rules! draw_case {
    ($ident:ident, $name:literal) => {
        #[test]
        fn $ident() {
            if let Err(e) = run_case($name) {
                panic!("{e}");
            }
        }
    };
}

draw_case!(noisy_count_draws, "noisy-count-draws");
draw_case!(noisy_count_7_draws, "noisy-count-7-draws");

fn draws_dir() -> PathBuf {
    crate_dir().join("fixtures").join("draws")
}

struct Expected {
    kind: &'static str,
    arg: f64,
    value: f64,
    int: bool,
}

fn num(e: &Sexp) -> Option<f64> {
    match e {
        Sexp::Int(i) => Some(*i as f64),
        Sexp::Float(x) => Some(*x),
        _ => None,
    }
}

fn parse_expected(path: &Path) -> Result<Vec<Expected>, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (n, line) in src.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let forms = parse_all(line).map_err(|e| format!("{}:{}: {e}", path.display(), n + 1))?;
        let Sexp::List(items) = &forms[0] else {
            return Err(format!("{}:{}: not a list", path.display(), n + 1));
        };
        if items.len() != 3 {
            return Err(format!(
                "{}:{}: want (kind arg value)",
                path.display(),
                n + 1
            ));
        }
        let kind = match items[0].as_sym().map(|s| s.trim_start_matches(':')) {
            Some("RANDOM") => "random",
            Some("NOISE") => "noise",
            got => return Err(format!("{}:{}: bad kind {got:?}", path.display(), n + 1)),
        };
        let arg = num(&items[1]).ok_or_else(|| format!("bad arg: {line}"))?;
        let value = num(&items[2]).ok_or_else(|| format!("bad value: {line}"))?;
        let int = kind == "random" && matches!(items[1], Sexp::Int(_));
        out.push(Expected {
            kind,
            arg,
            value,
            int,
        });
    }
    Ok(out)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * (1.0 + b.abs())
}

fn run_case(name: &str) -> Result<(), String> {
    let dir = draws_dir();
    let expected_path = dir.join(format!("{name}.draws.sexp"));
    let expected = parse_expected(&expected_path)?;
    let meta_path = dir.join(format!("{name}.meta"));
    let meta =
        fs::read_to_string(&meta_path).map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let model = meta_value(&meta, "model")?;
    let seed = parse_seed_str(&meta_value(&meta, "seed")?)?;
    let run_time: f64 = meta_value(&meta, "run")?
        .parse()
        .map_err(|_| format!("{name}: bad run time"))?;

    let path = models_dir().join(model.replace(';', std::path::MAIN_SEPARATOR_STR));
    let defs = load_models(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let def = defs
        .last()
        .ok_or_else(|| format!("{name}: no define-model in {}", path.display()))?;
    let mut eng = Engine::new(def.clone(), seed, 0.0, DeviceSetup::default())
        .map_err(|e| format!("{name}: engine error: {e}"))?;

    if let Ok(settings) = meta_value(&meta, "settings") {
        if !settings.eq_ignore_ascii_case("nil") {
            let forms = parse_all(&settings)
                .map_err(|e| format!("{name}: bad settings {settings:?}: {e}"))?;
            let Sexp::List(items) = forms
                .first()
                .ok_or_else(|| format!("{name}: empty settings"))?
            else {
                return Err(format!("{name}: settings is not a list"));
            };
            for pair in items.chunks(2) {
                let [key, value] = pair else {
                    return Err(format!("{name}: odd settings plist"));
                };
                let key = key
                    .as_sym()
                    .ok_or_else(|| format!("{name}: settings key is not a symbol"))?;
                eng.apply_sgp(key, value)
                    .map_err(|e| format!("{name}: apply {key}: {e}"))?;
            }
        }
    }

    eng.start_rng_log();
    eng.run_secs(run_time)
        .map_err(|e| format!("{name}: engine error: {e}"))?;
    let actual = eng.take_rng_log();

    for (i, (want, got)) in expected.iter().zip(actual.iter()).enumerate() {
        if want.kind != got.kind
            || want.int != got.int
            || !close(want.arg, got.arg)
            || !close(want.value, got.value)
        {
            return Err(format!(
                "{name}: draw {i} differs\n  want ({:?} arg={} value={} int={})\n  got  ({:?} arg={} value={} int={})",
                want.kind, want.arg, want.value, want.int, got.kind, got.arg, got.value, got.int
            ));
        }
    }
    if expected.len() != actual.len() {
        return Err(format!(
            "{name}: draw count differs: want {} got {}",
            expected.len(),
            actual.len()
        ));
    }
    Ok(())
}
