//! case: tutorial-unit6-bst-real-encode-line-goal-and-read-done-productions
//! ref:  tutorial-unit6-bst-real-encode-line-goal-and-read-done-productions.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-real-encode-line-goal-and-read-done-productions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit6_bst_real_encode_line_goal_and_read_done_productions() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-real-encode-line-goal-and-read-done-productions")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "pp".into(),
            args: crate::sexps(r#"(ENCODE-LINE-GOAL READ-DONE)"#),
        }])
        .check()
}
