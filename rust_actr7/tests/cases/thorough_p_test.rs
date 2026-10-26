//! case: thorough-p-test
//! ref:  thorough-p-test.ref
//! lisp: actr7.x/test-models/thorough-p-test.lisp

use crate::Case;

#[test]
#[ignore = "arbitrary-Lisp command stub (pending translation to rust)"]
fn thorough_p_test() -> Result<(), String> {
    Case::new("thorough-p-test").seed((100, 0)).check()
}
