//! case: tutorial-unit8-categorize-imaginal-chunk-with-feature
//! ref:  tutorial-unit8-categorize-imaginal-chunk-with-feature.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-imaginal-chunk-with-feature.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_categorize_imaginal_chunk_with_feature() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-imaginal-chunk-with-feature")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Command {
                name: "mod-buffer-chunk".into(),
                args: crate::sexps(r#"(IMAGINAL (EH LARGE))"#),
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
