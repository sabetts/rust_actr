//! case: tutorial-unit7modeling-ul-issues-first-run-with-ult
//! ref:  tutorial-unit7modeling-ul-issues-first-run-with-ult.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-first-run-with-ult.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_first_run_with_ult() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-first-run-with-ult")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Sgp(":ULT".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 5.05,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
