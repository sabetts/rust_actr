//! case: tutorial-unit8-simple-dynamic-retrieve-second-step-buffers
//! ref:  tutorial-unit8-simple-dynamic-retrieve-second-step-buffers.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-retrieve-second-step-buffers.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_dynamic_retrieve_second_step_buffers() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-dynamic-retrieve-second-step-buffers")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PBreak(vec!["RETRIEVE-SECOND-STEP".into()]),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into(), "RETRIEVAL".into()]),
        ])
        .check()
}
