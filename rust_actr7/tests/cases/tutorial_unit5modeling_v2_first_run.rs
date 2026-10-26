//! case: tutorial-unit5modeling-v2-first-run
//! ref:  tutorial-unit5modeling-v2-first-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v2-first-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v2_first_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v2-first-run")
        .models(&["test-models;models;declarative-issues-v2.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
