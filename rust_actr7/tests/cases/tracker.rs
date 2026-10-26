//! case: tracker
//! ref:  tracker.ref
//! lisp: actr7.x/test-models/tracker.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tracker() -> Result<(), String> {
    Case::new("tracker")
        .models(&["extras;tracker;simple.lisp"])
        .seed((57531033409, 0))
        .steps(vec![
            Step::Reset,
            Step::TrackerSetup(vec![
                ("SCREEN-X".into(), crate::sexp(r#"100"#)),
                ("SCREEN-Y".into(), crate::sexp(r#"100"#)),
                ("PROMPT".into(), crate::sexp(r#"CHOOSE"#)),
            ]),
            Step::Run {
                secs: 40.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
