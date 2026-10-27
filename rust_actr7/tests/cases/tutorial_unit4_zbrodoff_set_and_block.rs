//! case: tutorial-unit4-zbrodoff-set-and-block
//! ref:  tutorial-unit4-zbrodoff-set-and-block.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-set-and-block.lisp

use crate::Case;
use actr_engine::{Step, ZbrodoffMode};

#[test]
fn tutorial_unit4_zbrodoff_set_and_block() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-set-and-block")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Zbrodoff(ZbrodoffMode::SetAndBlock)])
        .check()
}
