//! case: tutorial-unit5code-siegler-experiment-new-rt
//! ref:  tutorial-unit5code-siegler-experiment-new-rt.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-new-rt.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_new_rt() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-new-rt")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 302,
            sets: 100,
            rt: Some(-0.7),
            mp: None,
            ans: None,
            esc: None,
            fact_base: Some(0.0),
            all_base: None,
        }])
        .check()
}
