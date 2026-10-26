//! case: tutorial-unit5modeling-v2-show-queue
//! ref:  tutorial-unit5modeling-v2-show-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v2-show-queue.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v2_show_queue() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v2-show-queue")
        .models(&["test-models;models;declarative-issues-v2.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Run {
                secs: 10.0,
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
