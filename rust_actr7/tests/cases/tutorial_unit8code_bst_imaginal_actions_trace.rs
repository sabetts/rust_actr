//! case: tutorial-unit8code-bst-imaginal-actions-trace
//! ref:  tutorial-unit8code-bst-imaginal-actions-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit8code-bst-imaginal-actions-trace.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, production, Projection};
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    armed: bool,
    collected: u32,
    lines: Rc<RefCell<Projection>>,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if !self.armed {
            if action(ev, "production-fired") && production(ev, "encode-line-current") {
                self.armed = true;
                return PreResult {
                    verdict: Verdict::Break,
                    trace: Some(false),
                };
            }
            return PreResult::trace(false);
        }
        PreResult::trace(false)
    }

    fn post(&mut self, line: &str, _ev: &EventView) {
        if !self.armed || self.collected >= 15 {
            return;
        }
        self.collected += 1;
        let lower = line.to_ascii_lowercase();
        let keep = (lower.contains("imaginal") && !lower.contains("mod-buffer-chunk"))
            || lower.contains("encode-line-current");
        self.lines.borrow_mut().record(line, keep, true);
    }
}

#[test]
fn tutorial_unit8code_bst_imaginal_actions_trace() -> Result<(), String> {
    let lines = Rc::new(RefCell::new(Projection::default()));
    let filter = Filter {
        lines: lines.clone(),
        ..Filter::default()
    };
    hooks::HookCase::new("tutorial-unit8code-bst-imaginal-actions-trace")
        .models(&[
            "tutorial;lisp;bst-ppm.lisp",
            "tutorial;unit8;bst-ppm-model.lisp",
        ])
        .seed((100, 1))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 60.0,
                trace: false,
                width: 22,
            },
            Step::RunNEvents {
                count: 15,
                trace: false,
                width: 22,
            },
        ])
        .check_projection(filter, lines, "\n", false)
}
