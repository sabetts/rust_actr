//! case: tutorial-unit7modeling-ul-issues-initial-long-term-spp
//! ref:  tutorial-unit7modeling-ul-issues-initial-long-term-spp.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-initial-long-term-spp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_initial_long_term_spp() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-initial-long-term-spp")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 700.0,
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
