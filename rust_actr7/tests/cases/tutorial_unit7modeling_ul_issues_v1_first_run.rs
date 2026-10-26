//! case: tutorial-unit7modeling-ul-issues-v1-first-run
//! ref:  tutorial-unit7modeling-ul-issues-v1-first-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-ul-issues-v1-first-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_ul_issues_v1_first_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-ul-issues-v1-first-run")
        .models(&[
            "tutorial;lisp;ul-issues.lisp",
            "tutorial;unit7;utility-learning-issues-model.lisp",
            "test-models;models;utility-learning-issues-model-v1.lisp",
        ])
        .seed((100, 2))
        .steps(vec![
            Step::Run {
                secs: 5.05,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(r#"(ISA CHOOSE)"#)),
        ])
        .check()
}
