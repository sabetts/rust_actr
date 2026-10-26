//! case: test-bad-productions
//! ref:  test-bad-productions.ref
//! lisp: actr7.x/test-models/test-bad-productions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn test_bad_productions() -> Result<(), String> {
    Case::new("test-bad-productions")
        .models(&["test-models;models;test-bad-productions.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
