//! case: tutorial-unit1modeling-broken-addition-v4-final-goal
//! ref:  tutorial-unit1modeling-broken-addition-v4-final-goal.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v4-final-goal.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v4_final_goal() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v4-final-goal")
        .models(&["test-models;models;broken-addition-v4.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (buffer-chunk goal)".into()),
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
