//! case: tutorial-unit3-sperling-trial-trace
//! ref:  tutorial-unit3-sperling-trial-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-trial-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3_sperling_trial_trace() -> Result<(), String> {
    Case::new("tutorial-unit3-sperling-trial-trace")
        .models(&["tutorial;unit3;sperling-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
