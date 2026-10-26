//! case: tutorial-unit8-semantic-dynamic-extend-warning
//! ref:  tutorial-unit8-semantic-dynamic-extend-warning.ref
//! lisp: actr7.x/test-models/tutorial-unit8-semantic-dynamic-extend-warning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_semantic_dynamic_extend_warning() -> Result<(), String> {
    Case::new("tutorial-unit8-semantic-dynamic-extend-warning")
        .models(&["tutorial;unit8;semantic-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "add-dm".into(),
            args: crate::sexps(r#"((P25 ISA PROPERTY OBJECT SHARK TEETH SHARP))"#),
        }])
        .check()
}
