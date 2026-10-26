//! case: tutorial-unit8-categorize-sample-dm-chunks
//! ref:  tutorial-unit8-categorize-sample-dm-chunks.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-sample-dm-chunks.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_categorize_sample_dm_chunks() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-sample-dm-chunks")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "dm".into(),
            args: crate::sexps(r#"(EXAMPLE4 EXAMPLE5)"#),
        }])
        .check()
}
