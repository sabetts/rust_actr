//! case: tutorial-unit5modeling-v10-sdm
//! ref:  tutorial-unit5modeling-v10-sdm.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v10-sdm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v10_sdm() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v10-sdm")
        .models(&["test-models;models;declarative-issues-v10.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":NCNAR".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (sdm - goal nil)".into()),
            Step::Sdm(vec!["-".into(), "GOAL".into(), "NIL".into()]),
        ])
        .check()
}
