//! case: tutorial-unit1-count-first-retrieval-chunk
//! ref:  tutorial-unit1-count-first-retrieval-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit1-count-first-retrieval-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_count_first_retrieval_chunk() -> Result<(), String> {
    Case::new("tutorial-unit1-count-first-retrieval-chunk")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.1,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["RETRIEVAL".into()]),
        ])
        .check()
}
