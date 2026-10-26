//! case: semantic
//! ref:  semantic.ref
//! lisp: actr7.x/test-models/semantic.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn semantic() -> Result<(), String> {
    Case::new("semantic")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(
                r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY ANIMAL)"#,
            )),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::GoalFocus(crate::chunk_desc(
                r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY FISH)"#,
            )),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
