//! case: tutorial-unit1code-semantic-buffer-specific
//! ref:  tutorial-unit1code-semantic-buffer-specific.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-buffer-specific.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_buffer_specific() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-buffer-specific")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::ModelOutput("? (buffer-chunk retrieval goal)".into()),
            Step::BufferChunk(vec!["RETRIEVAL".into(), "GOAL".into()]),
            Step::ModelOutput("(NIL GOAL-CHUNK0)".into()),
        ])
        .check()
}
