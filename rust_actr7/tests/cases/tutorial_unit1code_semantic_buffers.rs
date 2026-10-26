//! case: tutorial-unit1code-semantic-buffers
//! ref:  tutorial-unit1code-semantic-buffers.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-buffers.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_buffers() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-buffers")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Run {
                secs: 0.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (buffer-chunk)".into()),
            Step::Command {
                name: "buffer-chunk".into(),
                args: Vec::new(),
            },
            Step::ModelOutput(
                "((AURAL) (AURAL-LOCATION) (GOAL GOAL-CHUNK0) (IMAGINAL) (IMAGINAL-ACTION) (MANUAL) (PRODUCTION) (RETRIEVAL) (TEMPORAL) (VISUAL) (VISUAL-LOCATION) (VOCAL))"
                    .into(),
            ),
        ])
        .check()
}
