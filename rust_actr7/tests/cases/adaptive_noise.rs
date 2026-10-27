//! case: adaptive-noise
//! ref:  adaptive-noise.ref
//! lisp: actr7.x/test-models/adaptive-noise.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: `:BL-HOOK` / `:NOISE-HOOK`.
// The results depend on the model's adaptive-noise base-level and
// only-permanent-noise callbacks, which the engine does not evaluate.
#[ignore = "missing-feature: adaptive-noise hooks"]
fn adaptive_noise() -> Result<(), String> {
    Case::new("adaptive-noise")
        .models(&["extras;adaptive-noise;test-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":UAN".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 500.0, trace: true, width: 22 },
            Step::ModelOutput("results: ((C1 -1.3374588 -3.032042 3.809351)\n          (C1 -1.1684765 -1.7319753 3.2170875)\n          (C1 1.6049154 -0.3279923 0.20090656)\n          (C1 -0.27186614 -2.3613195 1.3124113)\n          (C1 0.09768587 -0.19217587 0.9069337))\n".into()),
            Step::Reset,
            Step::Sgp(":UAN".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":BL-HOOK".into(), crate::sexp(r#"ADAPTIVE-NOISE-BASE-LEVEL-ACTIVATION"#)),
            Step::Sgp(":NOISE-HOOK".into(), crate::sexp(r#"ONLY-PERMANENT-NOISE"#)),
            Step::Run { secs: 500.0, trace: true, width: 22 },
            Step::ModelOutput("results: ((C1 0.28426218 -0.89428306 0.7525693)\n          (C1 0.16818324 -0.23507369 0.8451989)\n          (C1 2.3208327 0.61271447 0.09819178)\n          (C1 0.6155778 -0.3737054 0.5403286)\n          (C1 0.8166429 0.5751059 0.4419127))\n".into()),
        ])
        .check()
}
