//! case: tutorial-unit5modeling-v6-run-p3
//! ref:  tutorial-unit5modeling-v6-run-p3.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v6-run-p3.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v6_run_p3() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v6-run-p3")
        .models(&["test-models;models;declarative-issues-v6.lisp"])
        .seed((100, 1))
        .steps(vec![Step::Run {
            secs: 29.931,
            trace: true,
            width: 22,
        }])
        .check()
}
