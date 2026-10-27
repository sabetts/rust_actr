//! case: tutorial-unit8-bst-experiment-data
//! ref:  tutorial-unit8-bst-experiment-data.ref
//! lisp: actr7.x/test-models/tutorial-unit8-bst-experiment-data.lisp

use crate::Case;
use actr_engine::{BstMode, Step};

#[test]
// Full reason: the shared BST driver recovers unit 6, but the unit 8 PPM model
// always chooses the over strategy here; its imaginal-action difference/PPM
// similarities still diverge from the Lisp ref.
#[ignore = "pending: BST PPM strategy-choice divergence"]
fn tutorial_unit8_bst_experiment_data() -> Result<(), String> {
    Case::new("tutorial-unit8-bst-experiment-data")
        .models(&[
            "tutorial;lisp;bst-ppm.lisp",
            "tutorial;unit8;bst-ppm-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Bst(BstMode::Ppm)])
        .check()
}
