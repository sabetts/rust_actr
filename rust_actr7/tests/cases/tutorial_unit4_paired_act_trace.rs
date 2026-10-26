//! case: tutorial-unit4-paired-act-trace
//! ref:  tutorial-unit4-paired-act-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-act-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit4_paired_act_trace() -> Result<(), String> {
    Case::new("tutorial-unit4-paired-act-trace")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
        ])
        .check()
}
