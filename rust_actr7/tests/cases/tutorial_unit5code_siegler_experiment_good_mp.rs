//! case: tutorial-unit5code-siegler-experiment-good-mp
//! ref:  tutorial-unit5code-siegler-experiment-good-mp.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-good-mp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_good_mp() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-good-mp")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 102,
            sets: 100,
            rt: Some(-10.0),
            mp: None,
            ans: None,
            esc: None,
            fact_base: Some(0.0),
            all_base: None,
        }])
        .check()
}
