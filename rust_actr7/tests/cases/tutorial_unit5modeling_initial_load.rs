//! case: tutorial-unit5modeling-initial-load
//! lisp: actr7.x/test-models/tutorial-unit5modeling-initial-load.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_initial_load() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-initial-load")
        .models(&["tutorial;unit5;declarative-issues.lisp"])
        .steps(vec![Step::Reset])
        .check()
}
