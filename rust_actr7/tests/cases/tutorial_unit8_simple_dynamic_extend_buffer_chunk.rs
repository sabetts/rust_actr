//! case: tutorial-unit8-simple-dynamic-extend-buffer-chunk
//! ref:  tutorial-unit8-simple-dynamic-extend-buffer-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-extend-buffer-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook conversion pending"]
fn tutorial_unit8_simple_dynamic_extend_buffer_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-dynamic-extend-buffer-chunk")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::RunNEvents {
                count: 4,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
