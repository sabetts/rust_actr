//! case: tutorial-unit5modeling-v1-sdp
//! ref:  tutorial-unit5modeling-v1-sdp.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v1-sdp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v1_sdp() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v1-sdp")
        .models(&["test-models;models;declarative-issues-v1.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 2.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("? (sdp v1 v2 v3)\n".into()),
            Step::ModelOutput(">>> actr.sdp('v1','v2','v3')".into()),
            Step::ModelOutput("".into()),
            Step::Command {
                name: "sdp".into(),
                args: crate::sexps(r#"(V1 V2 V3)"#),
            },
        ])
        .check()
}
