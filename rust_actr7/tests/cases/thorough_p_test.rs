//! case: thorough-p-test
//! ref:  thorough-p-test.ref
//! lisp: actr7.x/test-models/thorough-p-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn thorough_p_test() -> Result<(), String> {
    Case::new("thorough-p-test")
        .models(&["test-models;models;thorough-p-test.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Reset, Step::ThoroughP])
        .check()
}
