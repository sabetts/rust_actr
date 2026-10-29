//! case: nearest-vis-loc
//! ref:  nearest-vis-loc.ref
//! lisp: actr7.x/test-models/nearest-vis-loc.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn nearest_vis_loc() -> Result<(), String> {
    Case::new("nearest-vis-loc")
        .models(&["test-models;models;nearest-vis-loc.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::SetTaskCommandValue {
                name: "WHICH-ONE".into(),
                value: crate::sexp("CURRENT"),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::MoveAttention("START".into()),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::SetTaskCommandValue {
                name: "WHICH-ONE".into(),
                value: crate::sexp("CURRENT-X"),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::MoveAttention("START".into()),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::SetTaskCommandValue {
                name: "WHICH-ONE".into(),
                value: crate::sexp("CURRENT-Y"),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::MoveAttention("START".into()),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::SetTaskCommandValue {
                name: "WHICH-ONE".into(),
                value: crate::sexp("CLOCKWISE"),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::MoveAttention("START".into()),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::SetTaskCommandValue {
                name: "WHICH-ONE".into(),
                value: crate::sexp("COUNTERCLOCKWISE"),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::MoveAttention("START".into()),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
