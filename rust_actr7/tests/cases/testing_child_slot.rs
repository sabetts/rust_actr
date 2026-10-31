//! case: testing-child-slot
//! ref:  testing-child-slot.ref
//! lisp: actr7.x/test-models/testing-child-slot.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn testing_child_slot() -> Result<(), String> {
    Case::new("testing-child-slot")
        .models(&["test-models;models;child-slots.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":STYLE-WARNINGS".into(), crate::sexp(r#"NIL"#)),
            Step::TestingChildSlot,
        ])
        .check()
}
