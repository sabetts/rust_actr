//! case: paired
//! ref:  paired.ref
//! lisp: actr7.x/test-models/paired.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn paired() -> Result<(), String> {
    Case::new("paired")
        .models(&["tutorial;unit4;paired-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::RecordHistory(vec![
                "GOAL".into(),
                "VISUAL".into(),
                "VISUAL-LOCATION".into(),
                "IMAGINAL".into(),
                "RETRIEVAL".into(),
                "MANUAL".into(),
            ]),
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 5)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 6)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 8)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 10)),
            },
            Step::Sdm(vec!["ISA".into(), "PAIR".into()]),
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 5)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 6)),
            },
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 8)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 10)),
            },
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::ConsumeRng(2),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 6)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 7)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 8)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 9)),
            },
            Step::ConsumeRng(2),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 12)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 14)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 15)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((200, 17)),
            },
            Step::PredictBold,
        ])
        .check()
}
