//! case: tutorial-unit7modeling-ul-issues-v1-second-run
//! ref:  tutorial-unit7modeling-ul-issues-v1-second-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-v1-second-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_v1_second_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-v1-second-run")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
            "test-models;models;utility-learning-issues-model-v1.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 7.0,
                trace: false,
                width: 22,
            },
            Step::Run {
                secs: 7.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
