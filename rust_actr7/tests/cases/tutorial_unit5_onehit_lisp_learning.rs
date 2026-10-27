//! case: tutorial-unit5-onehit-lisp-learning
//! ref:  tutorial-unit5-onehit-lisp-learning.ref
//! lisp: actr7.x/test-models/tutorial-unit5-onehit-lisp-learning.lisp

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_lisp_learning() -> Result<(), String> {
    Case::new("tutorial-unit5-onehit-lisp-learning")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((101, 0))
        .steps(vec![Step::Onehit(OnehitMode::Learning { games: 50 })])
        .check()
}
