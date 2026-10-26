//! case: tutorial-unit6-bst-real-encode-line-goal-production
//! ref:  tutorial-unit6-bst-real-encode-line-goal-production.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-real-encode-line-goal-production.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit6_bst_real_encode_line_goal_production() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-real-encode-line-goal-production")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "pp".into(),
            args: crate::sexps(r#"(ENCODE-LINE-GOAL)"#),
        }])
        .check()
}
