//! case: tutorial-unit3modeling-v4-next-segment-trace
//! ref:  tutorial-unit3modeling-v4-next-segment-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v4-next-segment-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit3modeling_v4_next_segment_trace() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-v4-next-segment-trace")
        .models(&["tutorial;lisp;pm-issues.lisp", "tutorial;unit3;perceptual-motor-issues-model.lisp", "test-models;models;perceptual-motor-issues-model-v4.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "permute-list".into(), args: crate::sexps(r#"("a" "b" "c" "d" "e" "f" "g" "h" "i" "j" "k" "l" "m" "n" "o" "p" "q" "r" "s" "t" "u" "v" "w" "x" "y" "z")"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(27),
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::ModelOutput("".into()),
        ])
        .check()
}
