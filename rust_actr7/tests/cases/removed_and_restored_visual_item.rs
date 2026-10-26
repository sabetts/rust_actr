//! case: removed-and-restored-visual-item
//! ref:  removed-and-restored-visual-item.ref
//! lisp: actr7.x/test-models/removed-and-restored-visual-item.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn removed_and_restored_visual_item() -> Result<(), String> {
    Case::new("removed-and-restored-visual-item")
        .models(&["test-models;models;removed-and-restored-visual-item.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "permute-list".into(), args: crate::sexps(r#"("B" "C" "D" "F" "G" "H" "J" "K" "L" "M" "N" "P" "Q" "R" "S" "T" "V" "W" "X" "Y" "Z")"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(21),
            Step::RunNEvents { count: 3, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::BufferChunk(vec!["VISUAL".into()]),
            Step::Reset,
            Step::RunNEvents { count: 3, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::BufferChunk(vec!["VISUAL".into()]),
        ])
        .check()
}
