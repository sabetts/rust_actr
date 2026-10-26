//! case: tutorial-unit5-fan-imaginal-chunk-with-arg1
//! ref:  tutorial-unit5-fan-imaginal-chunk-with-arg1.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-imaginal-chunk-with-arg1.lisp

use crate::hooks::{self, action, buffer};
use actr_engine::{EventHook, EventView, PreResult, Step};

#[derive(Default)]
struct Filter {
    seen_mod: bool,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if self.seen_mod {
            return PreResult::BREAK;
        }
        if action(ev, "mod-buffer-chunk") && buffer(ev, "IMAGINAL") {
            self.seen_mod = true;
        }
        PreResult::CONTINUE
    }
}

#[test]
fn tutorial_unit5_fan_imaginal_chunk_with_arg1() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5-fan-imaginal-chunk-with-arg1")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .device("tutorial-unit5-fan-lawyer-store-trace")
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::Run {
                secs: 30.0,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check_segment(Filter::default())
}
