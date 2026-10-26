//! case: tutorial-unit4-paired-trace
//! ref:  tutorial-unit4-paired-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit4_paired_trace() -> Result<(), String> {
    Case::new("tutorial-unit4-paired-trace")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 5)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 6)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 8)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 10)),
            },
        ])
        .check()
}
