//! case: tutorial-unit5modeling-v7-p4-run
//! ref:  tutorial-unit5modeling-v7-p4-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v7-p4-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v7_p4_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v7-p4-run")
        .models(&["test-models;models;declarative-issues-v7.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 26.541,
                trace: false,
                width: 22,
            },
            Step::RunUntilTime {
                time: 30.06,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
