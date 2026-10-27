//! case: fan-saved-act
//! ref:  fan-saved-act.ref
//! lisp: actr7.x/test-models/fan-saved-act.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn fan_saved_act() -> Result<(), String> {
    Case::new("fan-saved-act")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":SACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-chunk-activation-trace".into(),
                args: crate::sexps(r#"(P13 585 T)"#),
            },
            Step::Command {
                name: "print-chunk-activation-trace".into(),
                args: crate::sexps(r#"(P1 585 T)"#),
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::Sgp(":SACT".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-activation-trace".into(),
                args: crate::sexps(r#"(585)"#),
            },
        ])
        .check()
}
