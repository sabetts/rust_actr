//! case: tutorial-unit1code-run-semantic
//! ref:  tutorial-unit1code-run-semantic.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-run-semantic.lisp

use crate::Case;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit1code_run_semantic() -> Result<(), String> {
    Case::new("tutorial-unit1code-run-semantic")
        .seed((100, 0))
        .check()
}
