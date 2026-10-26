//! case: tutorial-unit3modeling-v4-whynot
//! ref:  tutorial-unit3modeling-v4-whynot.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v4-whynot.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3modeling_v4_whynot() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-v4-whynot")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
            "test-models;models;perceptual-motor-issues-model-v4.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(27),
            Step::Run {
                secs: 10.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "whynot".into(),
                args: crate::sexps(r#"(RESPOND-PREVIOUS)"#),
            },
        ])
        .check()
}
