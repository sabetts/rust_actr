//! case: bind-test
//! ref:  bind-test.ref
//! lisp: actr7.x/test-models/bind-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn bind_test() -> Result<(), String> {
    Case::new("bind-test")
        .models(&["test-models;models;bind-test.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
