//! case: tutorial-unit2-demo2-visual-location-chunk
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-visual-location-chunk.lisp

use crate::hooks::{self, action, buffer};
use actr_engine::{EventHook, EventView, HookEffect, Step};

#[derive(Default)]
struct Filter {
    emitted: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn post(&mut self, _line: &str, ev: &EventView) {
        if !self.emitted
            && ev.time_ms == 50
            && action(ev, "set-buffer-chunk")
            && buffer(ev, "visual-location")
        {
            self.emitted = true;
            self.effects
                .push(HookEffect::PrintBufferChunk(vec!["VISUAL-LOCATION".into()]));
        }
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[test]
fn tutorial_unit2_demo2_visual_location_chunk() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit2-demo2-visual-location-chunk")
        .models(&[
            "tutorial;lisp;demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .seed((100, 0))
        .device("demo2")
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: false,
                width: 22,
            },
        ])
        .check_segment(Filter::default())
}
