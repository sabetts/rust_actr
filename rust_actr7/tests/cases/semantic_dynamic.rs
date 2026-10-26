//! case: semantic-dynamic
//! ref:  semantic-dynamic.ref
//! lisp: actr7.x/test-models/semantic-dynamic.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn semantic_dynamic() -> Result<(), String> {
    Case::new("semantic-dynamic")
        .models(&["tutorial;unit8;semantic-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::GoalFocus(crate::chunk_desc(r#"G1"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(r#"G2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(r#"G3"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(r#"G2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
