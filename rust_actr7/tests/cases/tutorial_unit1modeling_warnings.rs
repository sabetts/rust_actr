//! case: tutorial-unit1modeling-warnings
//! ref:  tutorial-unit1modeling-warnings.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-warnings.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_warnings() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-warnings")
        .models(&["test-models;models;tutorial-unit1modeling-warnings.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::EmitWarning("Creating chunk STARTING with no slots".into()),
            Step::EmitWarning("A retrieval event has been aborted by a new request".into()),
            Step::EmitWarning("Production TEST already exists and it is being redefined.".into()),
        ])
        .check()
}
