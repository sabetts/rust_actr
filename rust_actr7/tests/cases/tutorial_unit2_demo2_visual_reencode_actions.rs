//! case: tutorial-unit2-demo2-visual-reencode-actions
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-visual-reencode-actions.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, module};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = module(ev, "vision") && ev.time_ms == 970;
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn hook_tutorial_unit2_demo2_visual_reencode_actions() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit2-demo2-visual-reencode-actions")
        .models(&[
            "tutorial;lisp;demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .device("demo2")
        .check_projection(Filter { out: out.clone() }, out, "\n", false)
}
