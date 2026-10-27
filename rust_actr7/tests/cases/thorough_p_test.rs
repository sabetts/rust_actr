//! case: thorough-p-test
//! ref:  thorough-p-test.ref
//! lisp: actr7.x/test-models/thorough-p-test.lisp

use crate::Case;

#[test]
// Full reason: model and task sources are available; the 64 KB ref needs a
// generated driver over 39 goal states using buffer modification, whynot, and
// exact matrix formatting.
#[ignore = "pending: generated whynot driver (sources available)"]
fn thorough_p_test() -> Result<(), String> {
    Case::new("thorough-p-test").seed((100, 0)).check()
}
