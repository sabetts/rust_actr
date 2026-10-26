//! case: tutorial-unit2code-addition-buffer-status
//! ref:  tutorial-unit2code-addition-buffer-status.ref
//! lisp: actr7.x/test-models/tutorial-unit2code-addition-buffer-status.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_addition_buffer_status() -> Result<(), String> {
    Case::new("tutorial-unit2code-addition-buffer-status")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (buffer-status goal visual)".into()),
            Step::Command {
                name: "buffer-status".into(),
                args: crate::sexps(r#"(GOAL VISUAL)"#),
            },
            Step::ModelOutput("(GOAL VISUAL)".into()),
        ])
        .check()
}
