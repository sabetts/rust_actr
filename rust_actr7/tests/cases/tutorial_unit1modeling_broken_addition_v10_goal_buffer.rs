//! case: tutorial-unit1modeling-broken-addition-v10-goal-buffer
//! ref:  tutorial-unit1modeling-broken-addition-v10-goal-buffer.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v10-goal-buffer.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v10_goal_buffer() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v10-goal-buffer")
        .models(&["test-models;models;broken-addition-v10.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (buffer-chunk goal)".into()),
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
