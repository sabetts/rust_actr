//! case: paired-learning
//! ref:  paired-learning.ref
//! lisp: actr7.x/test-models/paired-learning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn paired_learning() -> Result<(), String> {
    Case::new("paired-learning")
        .models(&[
            "tutorial;unit4;paired-model.lisp",
            "tutorial;unit7;paired-learning-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
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
                seed: Some((1, 2)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 16)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 24)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 39)),
            },
            Step::ConsumeRng(2),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 52)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 69)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 80)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 101)),
            },
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"1.0"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 1)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 15)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 24)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 41)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 53)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 74)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 83)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 103)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 113)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 130)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 142)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 167)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 179)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 200)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 212)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 236)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 245)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 263)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 275)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((1, 294)),
            },
        ])
        .check()
}
