//! case: tutorial-unit5code-siegler-experiment-rt-and-base-level
//! ref:  tutorial-unit5code-siegler-experiment-rt-and-base-level.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-rt-and-base-level.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_rt_and_base_level() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-rt-and-base-level")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 102,
            sets: 100,
            rt: Some(-10.0),
            mp: Some(5.0),
            ans: Some(0.5),
            esc: None,
            fact_base: Some(0.0),
            all_base: None,
        }])
        .check()
}
