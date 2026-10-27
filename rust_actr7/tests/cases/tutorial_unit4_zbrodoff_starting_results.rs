//! case: tutorial-unit4-zbrodoff-starting-results
//! ref:  tutorial-unit4-zbrodoff-starting-results.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-starting-results.lisp

use crate::Case;
use actr_engine::{Step, ZbrodoffMode};

#[test]
fn tutorial_unit4_zbrodoff_starting_results() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-starting-results")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Zbrodoff(ZbrodoffMode::StartingResults)])
        .check()
}
