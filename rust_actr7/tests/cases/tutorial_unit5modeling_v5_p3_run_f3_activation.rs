//! case: tutorial-unit5modeling-v5-p3-run-f3-activation
//! ref:  tutorial-unit5modeling-v5-p3-run-f3-activation.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v5-p3-run-f3-activation.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v5_p3_run_f3_activation() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v5-p3-run-f3-activation")
        .models(&["test-models;models;declarative-issues-v5.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::RecordHistory(vec!["RETRIEVAL-HISTORY".into()]),
            Step::Sgp(
                ":RETRIEVAL-REQUEST-HOOK".into(),
                crate::sexp(r#"DM-REQUEST-RECORDER"#),
            ),
            Step::Sgp(
                ":RETRIEVAL-SET-HOOK".into(),
                crate::sexp(r#"DM-RETRIEVAL-SET-RECORDER"#),
            ),
            Step::Sgp(":SACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
            Step::Command {
                name: "print-chunk-activation-trace".into(),
                args: crate::sexps(r#"(F3 26591 T)"#),
            },
        ])
        .check()
}
