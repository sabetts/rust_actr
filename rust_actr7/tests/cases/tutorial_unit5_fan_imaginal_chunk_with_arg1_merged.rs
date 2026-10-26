//! case: tutorial-unit5-fan-imaginal-chunk-with-arg1-merged
//! ref:  tutorial-unit5-fan-imaginal-chunk-with-arg1-merged.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-imaginal-chunk-with-arg1-merged.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, PreResult, Step};

struct Filter;

impl EventHook for Filter {
    fn pre(&mut self, _ev: &EventView) -> PreResult {
        PreResult::CONTINUE
    }
}

#[test]
fn tutorial_unit5_fan_imaginal_chunk_with_arg1_merged() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5-fan-imaginal-chunk-with-arg1-merged")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .device("tutorial-unit5-fan-lawyer-store-trace")
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::RunUntilTime {
                time: 0.3,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check_segment(Filter)
}
