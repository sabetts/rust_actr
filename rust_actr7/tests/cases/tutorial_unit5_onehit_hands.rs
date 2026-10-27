//! case: tutorial_unit5_onehit_hands
//! lisp: (no stub)

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_hands() -> Result<(), String> {
    Case::new("tutorial_unit5_onehit_hands")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((1, 0))
        .steps(vec![Step::Onehit(OnehitMode::PythonHands {
            seed: (1, 0),
            hands: 4,
        })])
        .check()
}
