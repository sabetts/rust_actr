//! case: proc-display-with-pending
//! ref:  proc-display-with-pending.ref
//! lisp: actr7.x/test-models/proc-display-with-pending.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn proc_display_with_pending() -> Result<(), String> {
    Case::new("proc-display-with-pending")
        .models(&["tutorial;lisp;demo2.lisp", "tutorial;unit2;demo2-model.lisp"])
        .seed((123456, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 0.01, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 0.06, trace: true, width: 22 },
            Step::Command { name: "add-visicon-features".into(), args: crate::sexps(r#"((SCREEN-X 50 SCREEN-Y 30 COLOR GREEN) (SCREEN-X 30 SCREEN-Y 50 COLOR BLUE))"#) },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 0.01, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 0.02, trace: true, width: 22 },
            Step::Command { name: "delete-visicon-features".into(), args: crate::sexps(r#"(#:VISICON-ID1)"#) },
            Step::Command { name: "modify-visicon-features".into(), args: crate::sexps(r#"(VISICON-ID2 COLOR BLACK VALUE "changed")"#) },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
            Step::Run { secs: 0.01, trace: true, width: 22 },
            Step::Command { name: "print-visicon".into(), args: crate::sexps(r#"()"#) },
        ])
        .check()
}
