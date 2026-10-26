//! case: paired-learning-dynamic
//! ref:  paired-learning-dynamic.ref
//! lisp: actr7.x/test-models/paired-learning-dynamic.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn paired_learning_dynamic() -> Result<(), String> {
    Case::new("paired-learning-dynamic")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
            "tutorial;unit8;paired-dynamic-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:SEED (1 0))"#),
            ),
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(2),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::ConsumeRng(2),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
        ])
        .check()
}
