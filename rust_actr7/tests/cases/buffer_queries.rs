//! case: buffer-queries
//! ref:  buffer-queries.ref
//! lisp: actr7.x/test-models/buffer-queries.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn buffer_queries() -> Result<(), String> {
    Case::new("buffer-queries")
        .models(&["test-models;models;buffer-queries.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 1.0,
            trace: true,
            width: 22,
        }])
        .check()
}
