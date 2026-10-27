//! case: tutorial-unit5-onehit-lisp-blocks
//! ref:  tutorial-unit5-onehit-lisp-blocks.ref
//! lisp: actr7.x/test-models/tutorial-unit5-onehit-lisp-blocks.lisp

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_lisp_blocks() -> Result<(), String> {
    Case::new("tutorial-unit5-onehit-lisp-blocks")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((1, 0))
        .steps(vec![Step::Onehit(OnehitMode::Blocks {
            blocks: 2,
            hands: 5,
        })])
        .check()
}
