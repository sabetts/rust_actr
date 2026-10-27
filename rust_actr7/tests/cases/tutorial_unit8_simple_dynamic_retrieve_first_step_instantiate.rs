//! case: tutorial-unit8-simple-dynamic-retrieve-first-step-instantiate
//! ref:  tutorial-unit8-simple-dynamic-retrieve-first-step-instantiate.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-retrieve-first-step-instantiate.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_dynamic_retrieve_first_step_instantiate() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-dynamic-retrieve-first-step-instantiate")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "pbreak".into(),
                args: crate::sexps(r#"(RETRIEVE-FIRST-STEP)"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
        ])
        .check()
}
