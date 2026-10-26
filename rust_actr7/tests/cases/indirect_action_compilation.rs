//! case: indirect-action-compilation
//! ref:  indirect-action-compilation.ref
//! lisp: actr7.x/test-models/indirect-action-compilation.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn indirect_action_compilation() -> Result<(), String> {
    Case::new("indirect-action-compilation")
        .models(&["test-models;models;indirect-action-compilation.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":CIA".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 0.2,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":CIA".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 0.2,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
