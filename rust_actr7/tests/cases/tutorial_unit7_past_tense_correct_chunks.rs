//! case: tutorial-unit7-past-tense-correct-chunks
//! ref:  tutorial-unit7-past-tense-correct-chunks.ref
//! lisp: actr7.x/test-models/tutorial-unit7-past-tense-correct-chunks.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit7_past_tense_correct_chunks() -> Result<(), String> {
    Case::new("tutorial-unit7-past-tense-correct-chunks")
        .models(&[
            "tutorial;lisp;past-tense.lisp",
            "tutorial;unit7;past-tense-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "pprint-chunks".into(),
                args: crate::sexps(r#"(PAST-TENSE0 PAST-TENSE1)"#),
            },
        ])
        .check()
}
