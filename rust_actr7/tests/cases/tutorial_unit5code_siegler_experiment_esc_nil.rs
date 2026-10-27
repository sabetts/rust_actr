//! case: tutorial-unit5code-siegler-experiment-esc-nil
//! ref:  tutorial-unit5code-siegler-experiment-esc-nil.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-esc-nil.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_esc_nil() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-esc-nil")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 102,
            sets: 1,
            rt: None,
            mp: None,
            ans: None,
            esc: Some(false),
            fact_base: None,
            all_base: None,
        }])
        .check()
}
