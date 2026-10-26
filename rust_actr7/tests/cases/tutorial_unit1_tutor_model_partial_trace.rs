//! case: tutorial-unit1-tutor-model-partial-trace
//! ref:  tutorial-unit1-tutor-model-partial-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1-tutor-model-partial-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_tutor_model_partial_trace() -> Result<(), String> {
    Case::new("tutorial-unit1-tutor-model-partial-trace")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::PDisable(vec![
                "PROCESS-CARRY".into(),
                "NO-CARRY".into(),
                "ADD-TENS-DONE".into(),
                "ADD-TENS-CARRY".into(),
            ]),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 11,
            },
        ])
        .check()
}
