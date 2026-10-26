//! case: fan-no-pm
//! ref:  fan-no-pm.ref
//! lisp: actr7.x/test-models/fan-no-pm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn fan_no_pm() -> Result<(), String> {
    Case::new("fan-no-pm")
        .models(&[
            "tutorial;lisp;fan-no-pm.lisp",
            "tutorial;unit5;fan-no-pm-model.lisp",
        ])
        .seed((1, 0))
        .steps(vec![
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:SEED (1 0))"#),
            ),
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Sdm(vec![
                "-".into(),
                "ARG1".into(),
                "NIL".into(),
                "-".into(),
                "ARG2".into(),
                "NIL".into(),
            ]),
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:SEED (1 0) :TESTING-FUNCTION TRACE-FAN-RUN)"#),
            ),
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:SEED (1 0) :TESTING-FUNCTION SET-TESTING-TRACES)"#),
            ),
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
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
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""cave""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""church""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""debutante""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""earl""#)),
                ("ARG2".into(), crate::sexp(r#""castle""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""earl""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""cave""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""church""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""debutante""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""earl""#)),
                ("ARG2".into(), crate::sexp(r#""castle""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""hippie""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""store""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""fireman""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""captain""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""bank""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""lawyer""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""earl""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-PERSON".into()]),
            Step::ModFocus(vec![
                ("ARG1".into(), crate::sexp(r#""giant""#)),
                ("ARG2".into(), crate::sexp(r#""park""#)),
            ]),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
