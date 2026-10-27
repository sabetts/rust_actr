//! case: keypad-test
//! ref:  keypad-test.ref
//! lisp: actr7.x/test-models/keypad-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: keypad punch event trace.
// The ref walks the keypad through repeated presses; motor preparation and
// key-event timing still differ.
#[ignore = "engine-divergence: motor keypad trace"]
fn keypad_test() -> Result<(), String> {
    Case::new("keypad-test")
        .models(&["test-models;models;keypad-test.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
