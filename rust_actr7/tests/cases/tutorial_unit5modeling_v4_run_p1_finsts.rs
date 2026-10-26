//! case: tutorial-unit5modeling-v4-run-p1-finsts
//! ref:  tutorial-unit5modeling-v4-run-p1-finsts.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v4-run-p1-finsts.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    started: bool,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let mut trace = None;
        if !self.started {
            self.started = true;
            trace = Some(false);
        }
        if ev.time_ms == 12259 && action(ev, "retrieved-chunk") {
            trace = Some(true);
        }
        PreResult {
            verdict: Verdict::Continue,
            trace,
        }
    }
}

#[test]
#[ignore = "event-hook driver + command effects pending"]
fn tutorial_unit5modeling_v4_run_p1_finsts() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v4-run-p1-finsts")
        .models(&["test-models;models;declarative-issues-v4.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 12.26,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("\n? (print-dm-finsts)".into()),
            Step::Command {
                name: "print-dm-finsts".into(),
                args: crate::sexps(r#"()"#),
            },
        ])
        .check_segment(Filter::default())
}
