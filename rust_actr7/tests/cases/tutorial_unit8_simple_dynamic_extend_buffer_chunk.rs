//! case: tutorial-unit8-simple-dynamic-extend-buffer-chunk
//! ref:  tutorial-unit8-simple-dynamic-extend-buffer-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-extend-buffer-chunk.lisp

use crate::hooks::{self, action, production};
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
            if action(ev, "production-fired") && production(ev, "retrieve-second-step") {
                self.phase = 1;
                self.count = 0;
                return PreResult {
                    verdict: Verdict::Break,
                    trace,
                };
            }
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        } else {
            self.count += 1;
            let trace = (self.count > 3).then_some(false);
            PreResult {
                verdict: Verdict::Continue,
                trace,
            }
        }
    }
}

#[test]
fn tutorial_unit8_simple_dynamic_extend_buffer_chunk() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit8-simple-dynamic-extend-buffer-chunk")
        .models(&["tutorial;unit8;simple-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: false,
                width: 22,
            },
            Step::RunNEvents {
                count: 4,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
