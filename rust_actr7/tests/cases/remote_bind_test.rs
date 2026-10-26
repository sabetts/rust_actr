//! case: remote-bind-test
//! ref:  remote-bind-test.ref
//! lisp: actr7.x/test-models/remote-bind-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn remote_bind_test() -> Result<(), String> {
    Case::new("remote-bind-test")
        .models(&["test-models;models;remote-bind-test.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
