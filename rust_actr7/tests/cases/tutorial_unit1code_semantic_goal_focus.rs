//! case: tutorial-unit1code-semantic-goal-focus
//! ref:  tutorial-unit1code-semantic-goal-focus.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-goal-focus.lisp

use crate::Case;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit1code_semantic_goal_focus() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-goal-focus")
        .seed((100, 0))
        .check()
}
