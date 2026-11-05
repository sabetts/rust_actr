//! case: adaptive-noise
//! ref:  adaptive-noise.ref
//! lisp: actr7.x/test-models/adaptive-noise.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn adaptive_noise() -> Result<(), String> {
    Case::new("adaptive-noise")
        .models(&["extras;adaptive-noise;test-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::Reset,
            // The test stub's `:starting-parameters (:seed (1 0))` is
            // re-applied on every reset.
            Step::Seed((1, 0)),
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":UAN".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::AdaptiveNoiseTrace,
            Step::Reset,
            Step::Seed((1, 0)),
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":UAN".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(
                ":BL-HOOK".into(),
                crate::sexp(r#"ADAPTIVE-NOISE-BASE-LEVEL-ACTIVATION"#),
            ),
            Step::Sgp(":NOISE-HOOK".into(), crate::sexp(r#"ONLY-PERMANENT-NOISE"#)),
            Step::AdaptiveNoiseTrace,
        ])
        .check()
}
