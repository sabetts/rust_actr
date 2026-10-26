//! case: tutorial-unit5modeling-v1-run
//! ref:  tutorial-unit5modeling-v1-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v1-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v1_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v1-run")
        .models(&["test-models;models;declarative-issues-v1.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 2.0,
            trace: true,
            width: 22,
        }])
        .check()
}
