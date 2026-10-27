//! case: audio-updates
//! ref:  audio-updates.ref
//! lisp: actr7.x/test-models/audio-updates.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: replaying `new-tone-sound` objects outside a run.
// The notification is replayed, but the sound object and its subsequent
// onset/stuff events are not (#204).
#[ignore = "missing-feature: audio sound-object replay"]
fn audio_updates() -> Result<(), String> {
    Case::new("audio-updates")
        .models(&["test-models;models;audio-updates.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::ModelOutput("\nTimes are in seconds".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::ModelOutput("\nTimes are in milliseconds".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
