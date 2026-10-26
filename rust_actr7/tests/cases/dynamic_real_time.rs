//! case: dynamic-real-time
//! ref:  dynamic-real-time.ref
//! lisp: actr7.x/test-models/dynamic-real-time.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn dynamic_real_time() -> Result<(), String> {
    Case::new("dynamic-real-time")
        .models(&["test-models;models;dynamic-real-time.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::DummyEvent(500),
            Step::Run {
                secs: 1.1,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::DummyEvent(500),
            Step::Run {
                secs: 1.1,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
