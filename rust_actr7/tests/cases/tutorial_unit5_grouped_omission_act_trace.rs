//! case: tutorial-unit5-grouped-omission-act-trace
//! lisp: actr7.x/test-models/tutorial-unit5-grouped-omission-act-trace.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, HistoryTraceSelect, HookEffect, Step};

#[derive(Default)]
struct Filter {
    started: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn post(&mut self, _line: &str, _ev: &EventView) {
        if !self.started {
            self.started = true;
            self.effects.push(HookEffect::Sgp {
                key: ":V".into(),
                value: crate::sexp("NIL"),
            });
            self.effects.push(HookEffect::Command {
                name: "record-history".into(),
                args: vec![crate::sexp(r#""retrieval-history""#)],
            });
        }
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[test]
fn tutorial_unit5_grouped_omission_act_trace() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5-grouped-omission-act-trace")
        .models(&[
            "tutorial;lisp;grouped.lisp",
            "tutorial;unit5;grouped-model.lisp",
        ])
        .seed((77093543977, 39813))
        .steps(vec![
            Step::ResetSilent,
            Step::Run {
                secs: 20.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::RetrievalHistoryTrace {
                at_ms: 15565,
                select: HistoryTraceSelect::Index(1),
            },
        ])
        .check_segment(Filter::default())
}
