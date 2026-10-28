//! case: tutorial-unit7modeling-pcomp-issues-v5-retrieval-time
//! ref:  tutorial-unit7modeling-pcomp-issues-v5-retrieval-time.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v5-retrieval-time.lisp

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    phase: u8,
    count: u32,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if self.phase == 0 {
            let trace = if ev.time_ms == 0 { Some(false) } else { None };
            if action(ev, "production-fired") {
                self.count += 1;
                if self.count == 40 {
                    self.phase = 1;
                    self.count = 0;
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
            self.count += 1;
            let trace = (self.count == 12).then_some(false);
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        }
    }
}

#[test]
fn tutorial_unit7modeling_pcomp_issues_v5_retrieval_time() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit7modeling-pcomp-issues-v5-retrieval-time")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v5.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: true,
                width: 22,
            },
            Step::RunNEvents {
                count: 13,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
