//! case: tutorial-unit5-fan-imaginal-chunk
//! ref:  tutorial-unit5-fan-imaginal-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-imaginal-chunk.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, PreResult, Step};

struct Filter;

impl EventHook for Filter {
    fn pre(&mut self, _ev: &EventView) -> PreResult {
        PreResult::CONTINUE
    }
}

#[test]
fn tutorial_unit5_fan_imaginal_chunk() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5-fan-imaginal-chunk")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .device("tutorial-unit5-fan-lawyer-store-trace")
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PDisable(vec![
                "RETRIEVE-FROM-PERSON".into(),
                "RETRIEVE-FROM-LOCATION".into(),
            ]),
            Step::Run {
                secs: 30.0,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check_segment(Filter)
}
