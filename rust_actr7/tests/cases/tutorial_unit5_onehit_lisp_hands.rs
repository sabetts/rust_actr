//! case: tutorial-unit5-onehit-lisp-hands
//! ref:  tutorial-unit5-onehit-lisp-hands.ref
//! lisp: actr7.x/test-models/tutorial-unit5-onehit-lisp-hands.lisp

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_lisp_hands() -> Result<(), String> {
    Case::new("tutorial-unit5-onehit-lisp-hands")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((1, 0))
        .steps(vec![Step::Onehit(OnehitMode::Hands { hands: 5 })])
        .check()
}
