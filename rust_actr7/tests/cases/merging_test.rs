//! case: merging-test
//! ref:  merging-test.ref
//! lisp: actr7.x/test-models/merging-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn merging_test() -> Result<(), String> {
    Case::new("merging-test")
        .models(&["test-models;models;merging-test.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 1.0,
            trace: true,
            width: 22,
        }])
        .check()
}
