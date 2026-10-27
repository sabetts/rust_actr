//! case: tutorial-unit7-past-tense-buffer-starts
//! ref:  tutorial-unit7-past-tense-buffer-starts.ref
//! lisp: actr7.x/test-models/tutorial-unit7-past-tense-buffer-starts.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7_past_tense_buffer_starts() -> Result<(), String> {
    Case::new("tutorial-unit7-past-tense-buffer-starts")
        .models(&[
            "tutorial;lisp;past-tense.lisp",
            "tutorial;unit7;past-tense-model.lisp",
        ])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "define-chunks".into(),
                args: crate::sexps(r#"((IMAGINAL-CHUNK0 ISA PAST-TENSE VERB DO))"#),
            },
            Step::Command {
                name: "set-buffer-chunk".into(),
                args: crate::sexps(r#"IMAGINAL IMAGINAL-CHUNK0"#),
            },
            Step::GoalFocus(crate::chunk_desc(r#"(ISA GOAL STATE START)"#)),
            Step::RunUntilTime {
                time: 0.001,
                trace: false,
                width: 22,
            },
            Step::PrintedBufferChunk(vec!["GOAL".into(), "IMAGINAL".into()]),
        ])
        .check()
}
