//! case: tutorial-unit1modeling-broken-addition-initial-warnings
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-initial-warnings.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_initial_warnings() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-initial-warnings")
        .models(&["test-models;models;broken-addition-v1.lisp"])
        .steps(vec![Step::Reset])
        .check()
}
