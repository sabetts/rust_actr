//! case: siegler
//! ref:  siegler.ref
//! lisp: actr7.x/test-models/siegler.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
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
