//! case: tutorial-unit1-count-trace
//! ref:  tutorial-unit1-count-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1-count-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_count_trace() -> Result<(), String> {
    Case::new("tutorial-unit1-count-trace")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 13,
        }])
        .check()
}
