//! case: precondition-test
//! ref:  precondition-test.ref
//! lisp: actr7.x/test-models/precondition-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn precondition_test() -> Result<(), String> {
    Case::new("precondition-test")
        .models(&["test-models;models;precondition-test.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
