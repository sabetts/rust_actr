//! case: tutorial-unit4-zbrodoff-experiment
//! ref:  tutorial-unit4-zbrodoff-experiment.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-experiment.lisp

use crate::Case;

#[test]
// Full reason: task and base model sources are available; this needs a Rust
// port of zbrodoff.lisp's trial loop, output-key callback, and summary
// formatting.
#[ignore = "pending: Zbrodoff task-driver port (sources available)"]
fn tutorial_unit4_zbrodoff_experiment() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-experiment")
        .seed((100, 0))
        .check()
}
