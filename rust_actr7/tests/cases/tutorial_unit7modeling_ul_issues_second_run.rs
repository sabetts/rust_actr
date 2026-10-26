//! case: tutorial-unit7modeling-ul-issues-second-run
//! ref:  tutorial-unit7modeling-ul-issues-second-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-second-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_second_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-second-run")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Run {
                secs: 6.95,
                trace: false,
                width: 22,
            },
            Step::Run {
                secs: 14.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
