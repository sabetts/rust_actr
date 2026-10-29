//! case: keypad-test
//! ref:  keypad-test.ref
//! lisp: actr7.x/test-models/keypad-test.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, HookEffect, Step};

#[derive(Default)]
struct KeyHandler {
    effects: Vec<HookEffect>,
}

impl EventHook for KeyHandler {
    fn post(&mut self, line: &str, _ev: &EventView) {
        if line.contains("output-key") {
            if let Some(key) = line.split_whitespace().last() {
                self.effects
                    .push(HookEffect::ModelOutput(format!("The key is \"{key}\"")));
            }
        }
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[test]
fn keypad_test() -> Result<(), String> {
    hooks::HookCase::new("keypad-test")
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
        .check_segment(KeyHandler::default())
}
