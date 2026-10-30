//! case: tutorial-unit8-bst-experiment-data
//! ref:  tutorial-unit8-bst-experiment-data.ref
//! lisp: actr7.x/test-models/tutorial-unit8-bst-experiment-data.lisp

use crate::Case;
use actr_engine::{BstMode, Step};

#[test]
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
