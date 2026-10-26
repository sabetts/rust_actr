//! case: tutorial-unit1modeling-broken-addition-v6-bad-goal
//! ref:  tutorial-unit1modeling-broken-addition-v6-bad-goal.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v6-bad-goal.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v6_bad_goal() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v6-bad-goal")
        .models(&["test-models;models;broken-addition-v6.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.05,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
