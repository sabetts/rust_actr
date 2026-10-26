//! case: tutorial-unit3modeling-v2-run
//! ref:  tutorial-unit3modeling-v2-run.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v2-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3modeling_v2_run() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-v2-run")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
            "test-models;models;perceptual-motor-issues-model-v2.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
