//! case: tutorial-unit5modeling-v10-sdp
//! ref:  tutorial-unit5modeling-v10-sdp.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v10-sdp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v10_sdp() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v10-sdp")
        .models(&["test-models;models;declarative-issues-v10.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":NCNAR".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput(">>> actr.sdp('old-context')".into()),
            Step::Command {
                name: "sdp".into(),
                args: crate::sexps(r#"(OLD-CONTEXT)"#),
            },
        ])
        .check()
}
