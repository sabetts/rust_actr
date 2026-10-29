//! case: tutorial-unit5code-siegler-experiment-fact-base-levels
//! ref:  tutorial-unit5code-siegler-experiment-fact-base-levels.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-fact-base-levels.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_fact_base_levels() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-fact-base-levels")
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
            fact_base: Some(0.5),
            all_base: None,
        }])
        .check()
}
