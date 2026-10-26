//! case: tutorial-unit8-categorize-attribute
//! ref:  tutorial-unit8-categorize-attribute.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-attribute.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, PreResult, Step, Verdict};

#[derive(Default)]
struct Filter {
    events: u32,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let trace = (ev.time_ms == 0).then_some(false);
        self.events += 1;
        PreResult {
            verdict: if self.events == 3 {
                Verdict::Break
            } else {
                Verdict::Continue
            },
            trace,
        }
    }
}

#[test]
fn tutorial_unit8_categorize_attribute() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit8-categorize-attribute")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::GoalFocus(crate::chunk_desc(
                "(name size state add-attribute value -0.9)",
            )),
            Step::Run {
                secs: 20.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::ModelOutput("? (categorize-attribute size -.9)".into()),
            Step::ModelOutput(">>> categorize.attribute('size',-.9)".into()),
            Step::ModelOutput(String::new()),
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check_segment(Filter::default())
}
