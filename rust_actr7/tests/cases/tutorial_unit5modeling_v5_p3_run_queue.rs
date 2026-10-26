//! case: tutorial-unit5modeling-v5-p3-run-queue
//! ref:  tutorial-unit5modeling-v5-p3-run-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v5-p3-run-queue.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter;

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let trace = (ev.time_ms > 26590 && action(ev, "clear-buffer")).then_some(true);
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
#[ignore = "event-hook driver + command effects pending"]
fn tutorial_unit5modeling_v5_p3_run_queue() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v5-p3-run-queue")
        .models(&["test-models;models;declarative-issues-v5.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
