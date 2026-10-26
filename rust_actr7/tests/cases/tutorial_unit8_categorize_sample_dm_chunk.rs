//! case: tutorial-unit8-categorize-sample-dm-chunk
//! ref:  tutorial-unit8-categorize-sample-dm-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-sample-dm-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_categorize_sample_dm_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-sample-dm-chunk")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "dm".into(),
            args: crate::sexps(r#"(EXAMPLE1)"#),
        }])
        .check()
}
