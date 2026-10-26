//! case: tutorial-unit2-demo2-find-unattended-letter-actions
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-find-unattended-letter-actions.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, production};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
    armed: bool,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let at_50 = ev.time_ms == 50;
        let keep = if at_50
            && action(ev, "production-fired")
            && production(ev, "find-unattended-letter")
        {
            self.armed = true;
            true
        } else if at_50 && self.armed && action(ev, "conflict-resolution") {
            self.armed = false;
            false
        } else {
            at_50 && self.armed
        };
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn hook_tutorial_unit2_demo2_find_unattended_letter_actions() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit2-demo2-find-unattended-letter-actions")
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
        .check_projection(
            Filter {
                out: out.clone(),
                armed: false,
            },
            out,
            "\n",
            false,
        )
}
