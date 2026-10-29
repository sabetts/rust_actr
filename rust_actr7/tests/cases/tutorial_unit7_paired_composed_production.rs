//! case: tutorial-unit7-paired-composed-production
//! ref:  tutorial-unit7-paired-composed-production.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-composed-production.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, HookEffect, PreResult, Step};

#[derive(Default)]
struct Filter {
    started: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn pre(&mut self, _ev: &EventView) -> PreResult {
        if !self.started {
            self.started = true;
            self.effects.push(HookEffect::Sgp {
                key: ":V".into(),
                value: crate::sexp("NIL"),
            });
            self.effects.push(HookEffect::Sgp {
                key: ":EPL".into(),
                value: crate::sexp("T"),
            });
        }
        PreResult::trace(false)
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

fn trial_run() -> Step {
    Step::RunFullTime {
        secs: 5.0,
        trace: false,
        width: 22,
        seed: Some((100, 0)),
    }
}

#[test]
fn tutorial_unit7_paired_composed_production() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit7-paired-composed-production")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"((("zinc" "9")))"#),
            },
            trial_run(),
            trial_run(),
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"((("zinc" "9")))"#),
            },
            trial_run(),
            trial_run(),
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::Command {
                name: "pp".into(),
                args: crate::sexps("PRODUCTION0"),
            },
        ])
        .check_segment(Filter::default())
}
