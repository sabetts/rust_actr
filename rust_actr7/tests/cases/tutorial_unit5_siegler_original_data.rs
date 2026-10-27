//! case: tutorial-unit5-siegler-original-data
//! ref:  tutorial-unit5-siegler-original-data.ref
//! lisp: actr7.x/test-models/tutorial-unit5-siegler-original-data.lisp

use crate::Case;

#[test]
// Full reason: no model is needed; *siegler-data* and display-results are
// available, but this needs a Rust formatter for the original-data table.
#[ignore = "pending: Siegler original-data formatter"]
fn tutorial_unit5_siegler_original_data() -> Result<(), String> {
    Case::new("tutorial-unit5-siegler-original-data")
        .seed((100, 0))
        .check()
}
