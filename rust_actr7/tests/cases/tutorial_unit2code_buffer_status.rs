//! case: tutorial_unit2code_buffer_status
//! ref:  tutorial_unit2code_buffer_status.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_buffer_status.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_buffer_status() -> Result<(), String> {
    Case::new("tutorial_unit2code_buffer_status")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput(">>> actr.buffer_status('goal','visual')".into()),
            Step::Command {
                name: "buffer-status".into(),
                args: crate::sexps(r#"(GOAL VISUAL)"#),
            },
            Step::ModelOutput("['GOAL', 'VISUAL']".into()),
        ])
        .check()
}
