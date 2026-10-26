//! case: tracking-test
//! ref:  tracking-test.ref
//! lisp: actr7.x/test-models/tracking-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tracking_test() -> Result<(), String> {
    Case::new("tracking-test")
        .models(&["test-models;models;tracking-test.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 3.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
