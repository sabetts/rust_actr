//! case: tutorial-unit6-chunks-with-default-slots
//! ref:  tutorial-unit6-chunks-with-default-slots.ref
//! lisp: actr7.x/test-models/tutorial-unit6-chunks-with-default-slots.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit6_chunks_with_default_slots() -> Result<(), String> {
    Case::new("tutorial-unit6-chunks-with-default-slots")
        .models(&["test-models;models;unit6-chunk-type-default-slots.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (define-chunks\n   (c1 isa a)\n   (c2 isa b)\n   (c3 isa b slot 3)\n   (c4 isa c)\n   (c5 isa c slot2 nil))".into()),
            Step::Command {
                name: "define-chunks".into(),
                args: crate::sexps(
                    r#"((C1 ISA A) (C2 ISA B) (C3 ISA B SLOT 3) (C4 ISA C) (C5 ISA C SLOT2 NIL))"#,
                ),
            },
            Step::ModelOutput("(C1 C2 C3 C4 C5)".into()),
            Step::ModelOutput("\n? (pprint-chunks c1 c2 c3 c4 c5)".into()),
            Step::Command {
                name: "pprint-chunks".into(),
                args: crate::sexps(r#"(C1 C2 C3 C4 C5)"#),
            },
            Step::ModelOutput("(C1 C2 C3 C4 C5)".into()),
        ])
        .check()
}
