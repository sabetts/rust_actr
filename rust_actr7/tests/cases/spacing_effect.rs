//! case: spacing-effect
//! ref:  spacing-effect.ref
//! lisp: actr7.x/test-models/spacing-effect.lisp

use std::fs;

use actr_engine::sexpr::{parse_all, Sexp};
use actr_engine::{run_model_source_steps, ChunkDesc, DeviceSetup, Step};

/// One subtest from the upstream `spacing-effect` task file. Each
/// `test-spacing-effect-N` starts with `(clear-all)`, so every section is a
/// fresh model (the ref concatenates their output).
fn trace(model: &str, steps: Vec<Step>) -> Result<String, String> {
    run_model_source_steps(model, (100, 0), &steps, &DeviceSetup::default())
        .map_err(|e| format!("engine error: {e}"))
}

fn run(secs: f64) -> Step {
    Step::RunUntilTime {
        time: secs,
        trace: true,
        width: 22,
    }
}

fn sgp(key: &str, value: &str) -> Step {
    Step::Sgp(key.into(), crate::sexp(value))
}

/// Test 1/2/3 use an empty `(define-model foo)`.
const EMPTY_MODEL: &str = "(define-model foo)";

/// Extract `(define-model foo ...)` from the `test-spacing-effect-4` defun in
/// the vendored task source, so the model stays upstream-owned.
fn spacing_model_source() -> Result<String, String> {
    let src = crate::source_text("extras;spacing-effect;spacing-effect.lisp")?;
    let forms = parse_all(&src).map_err(|e| format!("spacing-effect.lisp: {e}"))?;
    for form in &forms {
        let Sexp::List(items) = form else {
            continue;
        };
        if items.first().and_then(Sexp::as_sym) != Some("DEFUN") {
            continue;
        }
        let named_target = items
            .get(1)
            .and_then(Sexp::as_sym)
            .map(|n| n.eq_ignore_ascii_case("TEST-SPACING-EFFECT-4"))
            .unwrap_or(false);
        if !named_target {
            continue;
        }
        for sub in &items[2..] {
            let Sexp::List(parts) = sub else {
                continue;
            };
            if parts.first().and_then(Sexp::as_sym) == Some("DEFINE-MODEL") {
                return Ok(crate::sexp_show(sub));
            }
        }
    }
    Err("test-spacing-effect-4: no define-model form".to_string())
}

/// The shared `:eblse` setup of tests 1 and 2.
fn push_eblse_setup(steps: &mut Vec<Step>, intercept: &str, scale: &str) {
    steps.push(sgp(":ESC", "T"));
    steps.push(sgp(":EBLSE", "T"));
    steps.push(sgp(":V", "T"));
    steps.push(sgp(":ACT", "T"));
    steps.push(sgp(":SE-INTERCEPT", intercept));
    steps.push(sgp(":SE-SCALE", scale));
}

/// Test 1: `(compute-activation …)`, a fake retrieval (`set-buffer-chunk` +
/// `clear-buffer`), and the decay list after the final reference.
fn test_1() -> Result<String, String> {
    let mut steps = Vec::new();
    push_eblse_setup(&mut steps, "0.177", ".217");
    steps.push(Step::AddDm(vec![ChunkDesc::Named("G".into())]));
    for time in [126.0, 252.0, 4844.0] {
        steps.push(run(time));
        steps.push(Step::ComputeActivation("G".into()));
        steps.push(Step::SetBufferChunkQuiet {
            buffer: "RETRIEVAL".into(),
            desc: ChunkDesc::Named("G".into()),
        });
        steps.push(Step::ClearBuffer("RETRIEVAL".into()));
    }
    steps.push(run(5877.0));
    steps.push(Step::ComputeActivation("G".into()));
    steps.push(Step::ChunkDecays("G".into()));
    trace(EMPTY_MODEL, steps)
}

/// Test 2: reference history supplied directly through `sdp`.
fn test_2() -> Result<String, String> {
    let mut steps = Vec::new();
    push_eblse_setup(&mut steps, "0.177", ".217");
    steps.push(Step::AddDm(vec![ChunkDesc::Named("G".into())]));
    steps.push(Step::SetChunkReferences {
        chunk: "G".into(),
        refs: vec![4844.0, 252.0, 126.0, 0.0],
    });
    steps.push(run(5877.0));
    steps.push(Step::ComputeActivation("G".into()));
    steps.push(Step::ChunkDecays("G".into()));
    trace(EMPTY_MODEL, steps)
}

/// Test 3a: the default exponential decay equation with `:bll .5`.
fn test_3a() -> Result<String, String> {
    let steps = vec![
        sgp(":ESC", "T"),
        sgp(":V", "T"),
        sgp(":ACT", "T"),
        sgp(":BLL", ".5"),
        sgp(":OL", "NIL"),
        Step::AddDm(vec![ChunkDesc::Named("G".into())]),
        Step::SetChunkReferences {
            chunk: "G".into(),
            refs: vec![4844.0, 252.0, 126.0, 0.0],
        },
        run(5877.0),
        Step::ComputeActivation("G".into()),
    ];
    trace(EMPTY_MODEL, steps)
}

/// Test 3b: the spacing equation must reproduce 3a's activation when
/// `:se-scale` is its default 0 and `:se-intercept` 0.5.
fn test_3b() -> Result<String, String> {
    let steps = vec![
        sgp(":ESC", "T"),
        sgp(":EBLSE", "T"),
        sgp(":V", "T"),
        sgp(":ACT", "T"),
        Step::AddDm(vec![ChunkDesc::Named("G".into())]),
        Step::SetChunkReferences {
            chunk: "G".into(),
            refs: vec![4844.0, 252.0, 126.0, 0.0],
        },
        run(5877.0),
        Step::ComputeActivation("G".into()),
        Step::ChunkDecays("G".into()),
    ];
    trace(EMPTY_MODEL, steps)
}

/// Test 4: the full production model, `(sdp)` and the four decay lists.
fn test_4() -> Result<String, String> {
    let model = spacing_model_source()?;
    let steps = vec![
        run(5877.0),
        Step::SdpChunks(Vec::new()),
        Step::ChunkDecayLines(vec!["a".into(), "b".into(), "c".into(), "d".into()]),
    ];
    trace(&model, steps)
}

#[test]
fn spacing_effect() -> Result<(), String> {
    let ref_path = crate::refs_dir().join("spacing-effect.ref");
    let expected = crate::normalize(
        &fs::read_to_string(&ref_path).map_err(|e| format!("{}: {e}", ref_path.display()))?,
    );
    let mut combined = String::new();
    for part in [test_1()?, test_2()?, test_3a()?, test_3b()?, test_4()?] {
        combined.push_str(&part);
    }
    let actual = crate::normalize(&combined);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "spacing-effect: ref mismatch vs {}\n{}",
            ref_path.display(),
            crate::common::first_diff(&expected, &actual)
        ))
    }
}
