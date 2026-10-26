//! case: tutorial-unit5modeling-v9-p7-8-run
//! ref:  tutorial-unit5modeling-v9-p7-8-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-p7-8-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v9_p7_8_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-p7-8-run")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 27.43,
                trace: false,
                width: 22,
            },
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("".into()),
            Step::ModelOutput("? (sdm - state nil)".into()),
            Step::Sdm(vec!["-".into(), "STATE".into(), "NIL".into()]),
            Step::ModelOutput("\n>>> actr.sdm('-','goal','nil')".into()),
            Step::Sdm(vec!["-".into(), "GOAL".into(), "NIL".into()]),
        ])
        .check()
}
