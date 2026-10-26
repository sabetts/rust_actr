//! case: tutorial-unit8-simple-dynamic-cst-start
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-cst-start.lisp

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
            if action(ev, "conflict-resolution") {
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
            let mut trace = if self.resume_trace {
                self.resume_trace = false;
                Some(true)
            } else {
                None
            };

            if !action(ev, "conflict-resolution") {
                trace = Some(false);
            }
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        }
    }
}

#[test]
fn hook_tutorial_unit8_simple_dynamic_cst_start() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit8-simple-dynamic-cst-start")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp("NIL")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
