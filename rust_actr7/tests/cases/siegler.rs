//! case: siegler
//! ref:  siegler.ref
//! lisp: actr7.x/test-models/siegler.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn siegler() -> Result<(), String> {
    Case::new("siegler")
        .models(&[
            "tutorial;lisp;siegler.lisp",
            "tutorial;unit5;siegler-model.lisp",
        ])
        .seed((1, 0))
        .no_device()
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::SieglerExperiment {
                sets: 2,
                trace: true,
            },
        ])
        .check()
}
