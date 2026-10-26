//! case: tutorial-unit2code-addition-buffer-chunk
//! ref:  tutorial-unit2code-addition-buffer-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit2code-addition-buffer-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_addition_buffer_chunk() -> Result<(), String> {
    Case::new("tutorial-unit2code-addition-buffer-chunk")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::ModelOutput("? (buffer-chunk goal visual)".into()),
            Step::BufferChunk(vec!["GOAL".into(), "VISUAL".into()]),
            Step::ModelOutput("(GOAL-CHUNK0 NIL)".into()),
        ])
        .check()
}
