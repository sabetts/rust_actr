//! case: tutorial-unit4-zbrodoff-compare
//! ref:  tutorial-unit4-zbrodoff-compare.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-compare.lisp

use crate::Case;

#[test]
// Full reason: task and base model sources are available; this needs a Rust
// port of zbrodoff.lisp's trial loop, output-key callback, and summary
// formatting.
#[ignore = "pending: Zbrodoff task-driver port (sources available)"]
fn tutorial_unit4_zbrodoff_compare() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-compare")
        .seed((100, 0))
        .check()
}
