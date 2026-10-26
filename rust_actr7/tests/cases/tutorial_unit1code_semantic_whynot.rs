//! case: tutorial-unit1code-semantic-whynot
//! ref:  tutorial-unit1code-semantic-whynot.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-whynot.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_whynot() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-whynot")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (whynot direct-verify fail)".into()),
            Step::Command {
                name: "whynot".into(),
                args: crate::sexps(r#"(DIRECT-VERIFY FAIL)"#),
            },
            Step::ModelOutput("NIL".into()),
        ])
        .check()
}
