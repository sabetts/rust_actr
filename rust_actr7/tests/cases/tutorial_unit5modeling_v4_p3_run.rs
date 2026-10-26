//! case: tutorial-unit5modeling-v4-p3-run
//! ref:  tutorial-unit5modeling-v4-p3-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v4-p3-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v4_p3_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v4-p3-run")
        .models(&["test-models;models;declarative-issues-v4.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 30.0,
            trace: true,
            width: 22,
        }])
        .check()
}
