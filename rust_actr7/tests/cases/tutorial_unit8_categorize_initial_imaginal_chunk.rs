//! case: tutorial-unit8-categorize-initial-imaginal-chunk
//! ref:  tutorial-unit8-categorize-initial-imaginal-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-initial-imaginal-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_categorize_initial_imaginal_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-initial-imaginal-chunk")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::BufferChunk(vec!["IMAGINAL".into()])])
        .check()
}
