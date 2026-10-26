//! case: tutorial-unit1-semantic-warnings
//! ref:  tutorial-unit1-semantic-warnings.ref
//! lisp: actr7.x/test-models/tutorial-unit1-semantic-warnings.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit1_semantic_warnings() -> Result<(), String> {
    Case::new("tutorial-unit1-semantic-warnings")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Reset])
        .check()
}
