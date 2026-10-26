//! case: tutorial-unit5modeling-first-run
//! ref:  tutorial-unit5modeling-first-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-first-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_first_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-first-run")
        .models(&["tutorial;unit5;declarative-issues.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 0.14,
            trace: true,
            width: 22,
        }])
        .check()
}
