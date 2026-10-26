//! case: tutorial-unit1-tutor-model-trace
//! ref:  tutorial-unit1-tutor-model-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1-tutor-model-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_tutor_model_trace() -> Result<(), String> {
    Case::new("tutorial-unit1-tutor-model-trace")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 11,
        }])
        .check()
}
