//! case: siegler
//! ref:  siegler.ref
//! lisp: actr7.x/test-models/siegler.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `siegler-experiment` 2-set summary.
// The steps replay 12 runs but do not use SieglerDriver, so the ref's
// correlation/response matrix is never emitted; tone replay is also missing.
#[ignore = "driver-summary: Siegler experiment"]
fn siegler() -> Result<(), String> {
    Case::new("siegler")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .seed((1, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
