//! case: tutorial-unit5-siegler-experiment-original
//! ref:  tutorial-unit5-siegler-experiment-original.ref
//! lisp: actr7.x/test-models/tutorial-unit5-siegler-experiment-original.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5_siegler_experiment_original() -> Result<(), String> {
    Case::new("tutorial-unit5-siegler-experiment-original")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 110,
            sets: 100,
            rt: None,
            mp: None,
            ans: None,
            esc: None,
            fact_base: None,
            all_base: None,
        }])
        .check()
}
