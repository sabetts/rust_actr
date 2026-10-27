//! case: tutorial_unit5_onehit_learning
//! lisp: (no stub)

use crate::Case;
use actr_engine::{OnehitMode, Step};

#[test]
fn tutorial_unit5_onehit_learning() -> Result<(), String> {
    Case::new("tutorial_unit5_onehit_learning")
        .models(&[
            "tutorial;lisp;onehit.lisp",
            "tutorial;unit5;1hit-blackjack-model.lisp",
        ])
        .seed((3, 0))
        .steps(vec![Step::Onehit(OnehitMode::PythonLearning {
            first_seed: 101,
            games: 50,
        })])
        .check()
}
