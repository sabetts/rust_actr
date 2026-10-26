//! case: base-level-inhibition
//! ref:  base-level-inhibition.ref
//! lisp: actr7.x/test-models/base-level-inhibition.lisp

use crate::Case;

#[test]
#[ignore = "no run recorded"]
fn base_level_inhibition() -> Result<(), String> {
    Case::new("base-level-inhibition")
        .models(&["extras;base-level-inhibition;free-recall.lisp"])
        .seed((100, 0))
        .check()
}
