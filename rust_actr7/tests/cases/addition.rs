//! case: addition
//! ref:  addition.ref
//! lisp: actr7.x/test-models/addition.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn addition() -> Result<(), String> {
    Case::new("addition")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
