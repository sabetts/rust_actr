//! case: audio-updates
//! ref:  audio-updates.ref
//! lisp: actr7.x/test-models/audio-updates.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn audio_updates() -> Result<(), String> {
    let mut steps = vec![
        Step::Reset,
        Step::ModelOutput("\nTimes are in seconds".into()),
        Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
        Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
        Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
        Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
    ];
    for pass in 0..2 {
        if pass == 1 {
            steps.push(Step::Reset);
            steps.push(Step::ModelOutput("\nTimes are in milliseconds".into()));
            steps.push(Step::Sgp(":AUDIO-MS-TIMES".into(), crate::sexp(r#"T"#)));
        }
        steps.extend([
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(100 .1 .5)"#),
            },
            Step::Command {
                name: "new-tone-sound".into(),
                args: crate::sexps(r#"(1000 800 200 t)"#),
            },
            Step::Command {
                name: "new-ongoing-sound".into(),
                args: crate::sexps(r#"("first" .2 .5)"#),
            },
            Step::Command {
                name: "new-ongoing-sound".into(),
                args: crate::sexps(
                    r#"("other" 200 500 100 external word t (:both both 10) (:evt only-evt "'evt'") (:sound only-sound "'snd'"))"#,
                ),
            },
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-audicon".into(),
                args: Vec::new(),
            },
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-audicon".into(),
                args: Vec::new(),
            },
            Step::Command {
                name: "end-ongoing-sound".into(),
                args: crate::sexps(r#"(1)"#),
            },
            Step::Command {
                name: "schedule-event-relative".into(),
                args: crate::sexps(r#"(.7 end-ongoing-sound (2))"#),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-audicon".into(),
                args: Vec::new(),
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-audicon".into(),
                args: Vec::new(),
            },
        ]);
    }
    Case::new("audio-updates")
        .models(&["test-models;models;audio-updates.lisp"])
        .seed((100, 0))
        .steps(steps)
        .check()
}
