//! case: tutorial-unit8-categorize-final-result
//! ref:  tutorial-unit8-categorize-final-result.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-final-result.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_categorize_final_result() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-final-result")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::BufferChunk(vec!["IMAGINAL".into()])])
        .check()
}
