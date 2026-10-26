//! case: tutorial-unit1-semantic-g1-trace
//! ref:  tutorial-unit1-semantic-g1-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1-semantic-g1-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_semantic_g1_trace() -> Result<(), String> {
    Case::new("tutorial-unit1-semantic-g1-trace")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
