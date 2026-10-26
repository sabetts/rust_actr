//! case: time-size-check
//! ref:  time-size-check.ref
//! lisp: actr7.x/test-models/time-size-check.lisp

use crate::Case;

#[test]
#[ignore = "no run recorded"]
fn time_size_check() -> Result<(), String> {
    Case::new("time-size-check").seed((100, 0)).check()
}
