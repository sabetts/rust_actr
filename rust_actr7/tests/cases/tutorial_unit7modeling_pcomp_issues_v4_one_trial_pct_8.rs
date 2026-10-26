//! case: tutorial-unit7modeling-pcomp-issues-v4-one-trial-pct-8
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v4-one-trial-pct-8.lisp

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
            if action(ev, "production-fired") {
                self.seen += 1;
                if self.seen == 8 {
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

            if !action(ev, "production-fired") {
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
fn hook_tutorial_unit7modeling_pcomp_issues_v4_one_trial_pct_8() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit7modeling-pcomp-issues-v4-one-trial-pct-8")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v4.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp("NIL")),
            Step::Sgp(":EPL".into(), crate::sexp("T")),
            Step::Sgp(":PCT".into(), crate::sexp("T")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: true,
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
