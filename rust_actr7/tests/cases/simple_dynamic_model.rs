//! case: simple-dynamic-model
//! ref:  simple-dynamic-model.ref
//! lisp: actr7.x/test-models/simple-dynamic-model.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn simple_dynamic_model() -> Result<(), String> {
    Case::new("simple-dynamic-model")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
