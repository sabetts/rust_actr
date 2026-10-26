//! case: tutorial-unit5modeling-v6-p4-run-queue
//! ref:  tutorial-unit5modeling-v6-p4-run-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v6-p4-run-queue.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter;

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let trace = (ev.time_ms > 29980 && action(ev, "production-fired")).then_some(true);
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
fn tutorial_unit5modeling_v6_p4_run_queue() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v6-p4-run-queue")
        .models(&["test-models;models;declarative-issues-v6.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: false,
                width: 22,
            },
            Step::ModelOutput("".into()),
            Step::Command {
                name: "mp-show-queue".into(),
                args: Vec::new(),
            },
        ])
        .check_segment(Filter::default())
}
