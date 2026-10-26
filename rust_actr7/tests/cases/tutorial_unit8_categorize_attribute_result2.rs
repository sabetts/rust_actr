//! case: tutorial-unit8-categorize-attribute-result2
//! ref:  tutorial-unit8-categorize-attribute-result2.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-attribute-result2.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_categorize_attribute_result2() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-attribute-result2")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Command {
                name: "extend-possible-slots".into(),
                args: crate::sexps(r#"(S2)"#),
            },
            Step::Command {
                name: "mod-buffer-chunk".into(),
                args: crate::sexps(r#"(IMAGINAL (SIZE SMALL S2 LARGE))"#),
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
