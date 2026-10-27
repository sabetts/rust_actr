//! case: nearest-vis-loc
//! ref:  nearest-vis-loc.ref
//! lisp: actr7.x/test-models/nearest-vis-loc.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `:nearest` visual-location selection.
// The ref compares the chosen location and visicon across five reset/run
// cycles; nearest selection/visicon output still differs.
#[ignore = "engine-divergence: nearest visual-location"]
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
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-visicon".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
