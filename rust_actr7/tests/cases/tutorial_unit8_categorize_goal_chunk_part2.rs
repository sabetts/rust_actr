//! case: tutorial-unit8-categorize-goal-chunk-part2
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-goal-chunk-part2.lisp

use crate::hooks;
use actr_engine::{EventHook, EventView, HookEffect, PreResult, Step};

#[derive(Default)]
struct Filter {
    set: bool,
    effects: Vec<HookEffect>,
}

impl EventHook for Filter {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        if ev.time_ms == 0 {
            self.effects.push(HookEffect::Sgp {
                key: ":V".into(),
                value: crate::sexp("NIL"),
            });
        }
        if !self.set && ev.buffer_present("IMAGINAL") {
            self.set = true;
            self.effects.push(HookEffect::Command {
                name: "mod-buffer-chunk".into(),
                args: vec![crate::sexp("IMAGINAL"), crate::sexp("(CATEGORY 1)")],
            });
        }
        PreResult::CONTINUE
    }

    fn take_effects(&mut self) -> Vec<HookEffect> {
        std::mem::take(&mut self.effects)
    }
}

#[test]
fn tutorial_unit8_categorize_goal_chunk_part2() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit8-categorize-goal-chunk-part2")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::ResetSilent,
            Step::GoalFocus(crate::chunk_desc("(STATE CATEGORIZE)")),
            Step::Run {
                secs: 20.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check_segment(Filter::default())
}
