//! case: tutorial-unit8-bst-imaginal-chunk
//! ref:  tutorial-unit8-bst-imaginal-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-bst-imaginal-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_bst_imaginal_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-bst-imaginal-chunk")
        .models(&[
            "tutorial;lisp;bst-ppm.lisp",
            "tutorial;unit8;bst-ppm-model.lisp",
        ])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:SEED (100 1))"#),
            ),
            Step::Reset,
            Step::Sgp(":V".into(), crate::sexp(r#"NIL"#)),
            Step::PBreak(vec!["DECIDE-OVER".into(), "DECIDE-UNDER".into()]),
            Step::Run {
                secs: 60.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::PrintedBufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
