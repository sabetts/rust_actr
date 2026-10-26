//! case: tutorial-unit8-extend-warning
//! ref:  tutorial-unit8-extend-warning.ref
//! lisp: actr7.x/test-models/tutorial-unit8-extend-warning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit8_extend_warning() -> Result<(), String> {
    Case::new("tutorial-unit8-extend-warning")
        .models(&["test-models;models;tutorial-unit8-new-slot-warning-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Reset])
        .check()
}
