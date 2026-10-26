//! case: tutorial-unit5modeling-v1-medium-act-trace
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v1-medium-act-trace.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    seen: u32,
    resumed: bool,
    resume_trace: bool,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if !self.resumed {
            let trace = if ev.time_ms == 0 { Some(false) } else { None };
            if action(ev, "start-retrieval") {
                self.seen += 1;
                if self.seen == 1 {
                    self.resumed = true;
                    self.resume_trace = true;
                    return PreResult {
                        verdict: Verdict::Break,
                        trace,
                    };
                }
            }
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        } else {
            let trace = if self.resume_trace {
                self.resume_trace = false;
                Some(true)
            } else {
                None
            };
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        }
    }
}

#[test]
#[ignore = "engine divergence: base-level reference trace (pending)"]
fn hook_tutorial_unit5modeling_v1_medium_act_trace() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5modeling-v1-medium-act-trace")
        .models(&["test-models;models;declarative-issues-v1.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp("NIL")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
            Step::Sgp(":ACT".into(), crate::sexp("MEDIUM")),
            Step::Sgp(":ACT".into(), crate::sexp("T")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::RunNEvents {
                count: 1,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
