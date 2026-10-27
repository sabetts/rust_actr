//! case: tutorial-unit5modeling-v1-whynot-dm
//! ref:  tutorial-unit5modeling-v1-whynot-dm.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v1-whynot-dm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `whynot-dm`.
// The ref dumps each candidate's activation block and match/failure reason;
// the engine only has production-level `whynot`.
#[ignore = "missing-command: whynot-dm"]
fn tutorial_unit5modeling_v1_whynot_dm() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v1-whynot-dm")
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
            Step::ModelOutput("? (whynot-dm v1 v2 v3)".into()),
            Step::ModelOutput(">>> actr.whynot_dm('v1','v2','v3')".into()),
            Step::ModelOutput("".into()),
            Step::Command {
                name: "whynot-dm".into(),
                args: crate::sexps(r#"(V1 V2 V3)"#),
            },
        ])
        .check()
}
