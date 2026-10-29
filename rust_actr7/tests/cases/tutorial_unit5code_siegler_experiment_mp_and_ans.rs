//! case: tutorial-unit5code-siegler-experiment-mp-and-ans
//! ref:  tutorial-unit5code-siegler-experiment-mp-and-ans.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-mp-and-ans.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_siegler_experiment_mp_and_ans() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-mp-and-ans")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .steps(vec![Step::SieglerDriver {
            first_seed: 102,
            sets: 100,
            rt: Some(0.0),
            mp: Some(5.0),
            ans: Some(0.5),
            esc: None,
            fact_base: None,
            all_base: Some(0.0),
        }])
        .check()
}
