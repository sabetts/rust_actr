//! case: tutorial-unit5modeling-v4-run-p3-act
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v4-run-p3-act.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter;

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let mut trace = None;
        if ev.time_ms == 0 {
            trace = Some(false);
        }
        if ev.time_ms > 26590 {
            trace = Some(true);
        }
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
fn hook_tutorial_unit5modeling_v4_run_p3_act() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v4-run-p3-act")
        .models(&["test-models;models;declarative-issues-v4.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp("MEDIUM")),
            Step::Sgp(":ACT".into(), crate::sexp("T")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::Run {
                secs: 26.591,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
