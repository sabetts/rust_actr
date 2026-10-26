//! case: tutorial-unit3-sperling-remaining-audio-trace
//! ref:  tutorial-unit3-sperling-remaining-audio-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-remaining-audio-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook system-parameter effects pending"]
fn tutorial_unit3_sperling_remaining_audio_trace() -> Result<(), String> {
    Case::new("tutorial-unit3-sperling-remaining-audio-trace")
        .models(&["tutorial;lisp;sperling.lisp", "tutorial;unit3;sperling-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "permute-list".into(), args: crate::sexps(r#"("B" "C" "D" "F" "G" "H" "J" "K" "L" "M" "N" "P" "Q" "R" "S" "T" "V" "W" "X" "Y" "Z")"#) },
            Step::Command { name: "new-tone-sound".into(), args: crate::sexps(r#"(1000 0.5 0.15)"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(23),
            Step::Run { secs: 30.0, trace: true, width: 22 },
            Step::ModelOutput("".into()),
        ])
        .check()
}
