//! case: tutorial-unit2-demo2-encode-letter-imaginal-actions
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-encode-letter-imaginal-actions.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, buffer, module, production};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = ev.time_ms < 450
            && ((action(ev, "production-fired") && production(ev, "encode-letter"))
                || (action(ev, "module-request") && buffer(ev, "imaginal"))
                || (action(ev, "clear-buffer") && buffer(ev, "imaginal"))
                || module(ev, "imaginal"));
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn hook_tutorial_unit2_demo2_encode_letter_imaginal_actions() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit2-demo2-encode-letter-imaginal-actions")
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
        .check_projection(Filter { out: out.clone() }, out, "\n...\n", false)
}
