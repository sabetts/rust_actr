//! case: tutorial-unit4-zbrodoff-compare
//! ref:  tutorial-unit4-zbrodoff-compare.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-compare.lisp

use crate::Case;
use actr_engine::{Step, ZbrodoffMode};

#[test]
fn tutorial_unit4_zbrodoff_compare() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-compare")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Zbrodoff(ZbrodoffMode::Compare)])
        .check()
}
