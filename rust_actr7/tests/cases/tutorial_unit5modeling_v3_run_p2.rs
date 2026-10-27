//! case: tutorial-unit5modeling-v3-run-p2
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v3-run-p2.lisp

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
        if ev.time_ms > 12300 {
            trace = Some(true);
        }
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
fn hook_tutorial_unit5modeling_v3_run_p2() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v3-run-p2")
        .models(&["test-models;models;declarative-issues-v3.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp("T")),
            Step::Sgp(":ACT".into(), crate::sexp("T")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::Run {
                secs: 19.517,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
