//! case: count
//! ref:  count.ref
//! lisp: actr7.x/test-models/count.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn count() -> Result<(), String> {
    Case::new("count")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
