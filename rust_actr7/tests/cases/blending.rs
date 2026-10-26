//! case: blending
//! ref:  blending.ref
//! lisp: actr7.x/test-models/blending.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn blending() -> Result<(), String> {
    Case::new("blending")
        .models(&["extras;blending;blending-test-7.lisp"])
        .seed((1, 1))
        .steps(vec![Step::Run {
            secs: 5.0,
            trace: true,
            width: 22,
        }])
        .check()
}
