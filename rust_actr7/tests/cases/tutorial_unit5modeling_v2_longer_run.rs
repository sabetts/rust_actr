//! case: tutorial-unit5modeling-v2-longer-run
//! ref:  tutorial-unit5modeling-v2-longer-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v2-longer-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v2_longer_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v2-longer-run")
        .models(&["test-models;models;declarative-issues-v2.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 15.32,
            trace: true,
            width: 22,
        }])
        .check()
}
