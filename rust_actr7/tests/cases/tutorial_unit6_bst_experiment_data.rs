//! case: tutorial-unit6-bst-experiment-data
//! ref:  tutorial-unit6-bst-experiment-data.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-experiment-data.lisp

use crate::Case;
use actr_engine::{BstMode, Step};

#[test]
fn tutorial_unit6_bst_experiment_data() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-experiment-data")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Bst(BstMode::Experiment)])
        .check()
}
