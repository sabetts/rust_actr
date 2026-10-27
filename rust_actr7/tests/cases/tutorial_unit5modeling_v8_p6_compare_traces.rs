//! case: tutorial-unit5modeling-v8-p6-compare-traces
//! ref:  tutorial-unit5modeling-v8-p6-compare-traces.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v8-p6-compare-traces.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `print-chunk-activation-trace 1+2=3 27155 T`.
// The ref is the activation breakdown; the command is not implemented.
#[ignore = "missing-command: chunk activation trace"]
fn tutorial_unit5modeling_v8_p6_compare_traces() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v8-p6-compare-traces")
        .models(&["test-models;models;declarative-issues-v8.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 27.15,
                trace: false,
                width: 22,
            },
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
            Step::RunUntilTime {
                time: 28.421,
                trace: false,
                width: 22,
            },
            Step::Command {
                name: "print-chunk-activation-trace".into(),
                args: crate::sexps(r#"(1+2=3 27155 T)"#),
            },
        ])
        .check()
}
