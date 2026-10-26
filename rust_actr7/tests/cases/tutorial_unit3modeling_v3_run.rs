//! case: tutorial-unit3modeling-v3-run
//! ref:  tutorial-unit3modeling-v3-run.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v3-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3modeling_v3_run() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-v3-run")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
            "test-models;models;perceptual-motor-issues-model-v3.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (pm-issues-task \"next\")".into()),
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(27),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("(next T)".into()),
        ])
        .check()
}
