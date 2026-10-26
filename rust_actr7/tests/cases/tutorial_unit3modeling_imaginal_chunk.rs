//! case: tutorial-unit3modeling-imaginal-chunk
//! ref:  tutorial-unit3modeling-imaginal-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-imaginal-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3modeling_imaginal_chunk() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-imaginal-chunk")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(27),
            Step::RunUntilTime {
                time: 2.37,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
