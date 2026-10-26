//! case: tutorial-unit1-semantic-g1-final-goal
//! ref:  tutorial-unit1-semantic-g1-final-goal.ref
//! lisp: actr7.x/test-models/tutorial-unit1-semantic-g1-final-goal.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_semantic_g1_final_goal() -> Result<(), String> {
    Case::new("tutorial-unit1-semantic-g1-final-goal")
        .models(&["tutorial;unit1;semantic.lisp"])
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
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
