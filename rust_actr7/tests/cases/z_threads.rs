//! case: z-threads
//! ref:  z-threads.ref
//! lisp: actr7.x/test-models/z-threads.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn z_threads() -> Result<(), String> {
    Case::new("z-threads")
        .models(&["extras;threads;sample.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 2.0,
                trace: true,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
