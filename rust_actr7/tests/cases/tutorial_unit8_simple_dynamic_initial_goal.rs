//! case: tutorial-unit8-simple-dynamic-initial-goal
//! ref:  tutorial-unit8-simple-dynamic-initial-goal.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-initial-goal.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_dynamic_initial_goal() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-dynamic-initial-goal")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunNEvents {
                count: 2,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
