//! case: tutorial-unit5modeling-v4-p3-run-queue
//! ref:  tutorial-unit5modeling-v4-p3-run-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v4-p3-run-queue.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v4_p3_run_queue() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v4-p3-run-queue")
        .models(&["test-models;models;declarative-issues-v4.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Run {
                secs: 30.0,
                trace: false,
                width: 22,
            },
            Step::Command {
                name: "mp-show-queue".into(),
                args: Vec::new(),
            },
        ])
        .check()
}
