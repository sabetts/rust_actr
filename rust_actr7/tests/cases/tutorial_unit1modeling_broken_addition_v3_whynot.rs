//! case: tutorial-unit1modeling-broken-addition-v3-whynot
//! ref:  tutorial-unit1modeling-broken-addition-v3-whynot.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v3-whynot.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v3_whynot() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v3-whynot")
        .models(&["test-models;models;broken-addition-v3.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (whynot initialize-addition)".into()),
            Step::Command {
                name: "whynot".into(),
                args: crate::sexps(r#"(INITIALIZE-ADDITION)"#),
            },
        ])
        .check()
}
