//! case: tutorial-unit5modeling-v4-run-p2-finsts
//! ref:  tutorial-unit5modeling-v4-run-p2-finsts.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v4-run-p2-finsts.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    started: bool,
}

impl EventHook for Filter {
    fn pre(&mut self, _ev: &EventView) -> PreResult {
        let trace = if self.started {
            None
        } else {
            self.started = true;
            Some(false)
        };
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
fn tutorial_unit5modeling_v4_run_p2_finsts() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v4-run-p2-finsts")
        .models(&["test-models;models;declarative-issues-v4.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 26.541,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-dm-finsts".into(),
                args: Vec::new(),
            },
        ])
        .check_segment(Filter::default())
}
