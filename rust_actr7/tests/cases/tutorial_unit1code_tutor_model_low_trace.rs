//! case: tutorial-unit1code-tutor-model-low-trace
//! ref:  tutorial-unit1code-tutor-model-low-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-tutor-model-low-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_tutor_model_low_trace() -> Result<(), String> {
    Case::new("tutorial-unit1code-tutor-model-low-trace")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::SetDetail(crate::Detail::Low),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 11,
            },
        ])
        .check()
}
