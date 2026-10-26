//! case: tutorial-unit7modeling-ul-issues-v2-long-run-spp
//! ref:  tutorial-unit7modeling-ul-issues-v2-long-run-spp.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-v2-long-run-spp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_v2_long_run_spp() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-v2-long-run-spp")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
            "test-models;models;utility-learning-issues-model-v2.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 105.0,
                trace: false,
                width: 22,
            },
            Step::Command {
                name: "spp".into(),
                args: crate::sexps(r#"(CHOOSE-A CHOOSE-B)"#),
            },
        ])
        .check()
}
