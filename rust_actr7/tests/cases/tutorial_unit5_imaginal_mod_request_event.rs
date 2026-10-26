//! case: tutorial-unit5-imaginal-mod-request-event
//! ref:  tutorial-unit5-imaginal-mod-request-event.ref
//! lisp: actr7.x/test-models/tutorial-unit5-imaginal-mod-request-event.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook conversion pending"]
fn tutorial_unit5_imaginal_mod_request_event() -> Result<(), String> {
    Case::new("tutorial-unit5-imaginal-mod-request-event")
        .models(&["test-models;models;unit5-imaginal-mod-request-event.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.25,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("".into()),
        ])
        .check()
}
