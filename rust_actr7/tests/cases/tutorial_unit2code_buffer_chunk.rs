//! case: tutorial_unit2code_buffer_chunk
//! ref:  tutorial_unit2code_buffer_chunk.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_buffer_chunk.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_buffer_chunk() -> Result<(), String> {
    Case::new("tutorial_unit2code_buffer_chunk")
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
            Step::ModelOutput(">>> actr.buffer_chunk('goal','visual')".into()),
            Step::BufferChunk(vec!["GOAL".into(), "VISUAL".into()]),
            Step::ModelOutput("['GOAL-CHUNK0', None]".into()),
        ])
        .check()
}
