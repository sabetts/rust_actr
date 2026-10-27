//! case: tutorial-unit4-zbrodoff-experiment
//! ref:  tutorial-unit4-zbrodoff-experiment.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-experiment.lisp

use crate::Case;
use actr_engine::{Step, ZbrodoffMode};

#[test]
fn tutorial_unit4_zbrodoff_experiment() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-experiment")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::Zbrodoff(ZbrodoffMode::Experiment)])
        .check()
}
