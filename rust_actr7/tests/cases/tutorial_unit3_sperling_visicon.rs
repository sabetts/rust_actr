//! case: tutorial-unit3-sperling-visicon
//! ref:  tutorial-unit3-sperling-visicon.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-visicon.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3_sperling_visicon() -> Result<(), String> {
    Case::new("tutorial-unit3-sperling-visicon")
        .models(&["tutorial;lisp;sperling.lisp", "tutorial;unit3;sperling-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"("B" "C" "D" "F" "G" "H" "J" "K" "L" "M" "N" "P" "Q" "R" "S" "T" "V" "W" "X" "Y" "Z")"#),
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(1000 0.5 0.15)"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp("NIL")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::ConsumeRng(23),
            Step::RunUntilTime {
                time: 0.04,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::Command {
                name: "print-visicon".into(),
                args: Vec::new(),
            },
        ])
        .check()
}
