//! case: tutorial-unit4-paired-act-trace
//! ref:  tutorial-unit4-paired-act-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-act-trace.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, HookEffect, PreResult, Step};

#[derive(Default)]
struct Filter {
    disabled: bool,
    output_started: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if !self.disabled {
            self.disabled = true;
            self.effects.push(HookEffect::Sgp {
                key: ":ACT".into(),
                value: crate::sexp("T"),
            });
            return PreResult::trace(false);
        }
        if action(ev, "start-retrieval") {
            self.output_started = true;
            return PreResult::trace(true);
        }
        if self.output_started {
            self.output_started = false;
            self.effects.push(HookEffect::ModelOutput("...".into()));
            return PreResult::trace(false);
        }
        PreResult::CONTINUE
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[test]
fn tutorial_unit4_paired_act_trace() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit4-paired-act-trace")
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
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunFullTime {
                secs: 5.0,
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"((("zinc" "9")))"#),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
        ])
        .check_segment(Filter::default())
}
