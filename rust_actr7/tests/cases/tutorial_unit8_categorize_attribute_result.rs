//! case: tutorial-unit8-categorize-attribute-result
//! ref:  tutorial-unit8-categorize-attribute-result.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-attribute-result.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_categorize_attribute_result() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-attribute-result")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::BufferChunk(vec!["IMAGINAL".into()])])
        .check()
}
