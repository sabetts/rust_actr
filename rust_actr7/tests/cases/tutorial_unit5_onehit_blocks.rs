//! case: tutorial_unit5_onehit_blocks
//! lisp: (no stub)

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_blocks() -> Result<(), String> {
    Case::new("tutorial_unit5_onehit_blocks")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((3, 0))
        .steps(vec![Step::Onehit(OnehitMode::PythonBlocks {
            seed: (3, 0),
            blocks: 2,
            hands: 5,
        })])
        .check()
}
