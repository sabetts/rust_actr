//! case: unstuff-aural
//! ref:  unstuff-aural.ref
//! lisp: actr7.x/test-models/unstuff-aural.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn unstuff_aural() -> Result<(), String> {
    Case::new("unstuff-aural")
        .models(&["test-models;models;unstuff-aural.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = NIL overwrite = T modify yes".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = T overwrite = T modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 1 overwrite = T modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"2.5"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 2.5 overwrite = T modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = NIL overwrite = NIL modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = T overwrite = NIL modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 1 overwrite = NIL modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::GoalFocus(crate::chunk_desc(r#"FREE"#)),
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"2.5"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 2.5 overwrite = NIL modify yes".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = NIL overwrite = T modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = T overwrite = T modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 1 overwrite = T modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"2.5"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 2.5 overwrite = T modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = NIL overwrite = NIL modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = T overwrite = NIL modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 1 overwrite = NIL modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":UNSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"2.5"#)),
            Step::Sgp(":OVERSTUFF-AURAL-LOCATION".into(), crate::sexp(r#"NIL"#)),
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::ModelOutput("unstuff = 2.5 overwrite = NIL modify no".into()),
            Step::RunUntilTime {
                time: 2.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::RunUntilTime {
                time: 7.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 0.1)"#),
            },
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(200 0.5)"#),
            },
            Step::RunUntilTime {
                time: 12.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
