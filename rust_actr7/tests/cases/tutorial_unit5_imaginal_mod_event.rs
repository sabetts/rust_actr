//! case: tutorial-unit5-imaginal-mod-event
//! ref:  tutorial-unit5-imaginal-mod-event.ref
//! lisp: actr7.x/test-models/tutorial-unit5-imaginal-mod-event.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit5_imaginal_mod_event() -> Result<(), String> {
    Case::new("tutorial-unit5-imaginal-mod-event")
        .models(&["test-models;models;unit5-imaginal-mod-event.lisp"])
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
