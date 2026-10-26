//! case: tutorial-unit5-count-mod-request-goal-event
//! ref:  tutorial-unit5-count-mod-request-goal-event.ref
//! lisp: actr7.x/test-models/tutorial-unit5-count-mod-request-goal-event.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit5_count_mod_request_goal_event() -> Result<(), String> {
    Case::new("tutorial-unit5-count-mod-request-goal-event")
        .models(&["tutorial;unit1;count.lisp"])
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
            Step::ModelOutput("".into()),
        ])
        .check()
}
