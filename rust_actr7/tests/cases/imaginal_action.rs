//! case: imaginal-action
//! ref:  imaginal-action.ref
//! lisp: actr7.x/test-models/imaginal-action.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn imaginal_action() -> Result<(), String> {
    Case::new("imaginal-action")
        .models(&[
            "test-models;models;imaginal-action.lisp",
            "test-models;imaginal-action.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 5.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
