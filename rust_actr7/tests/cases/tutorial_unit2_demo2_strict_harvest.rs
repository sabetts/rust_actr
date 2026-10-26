//! case: tutorial-unit2-demo2-strict-harvest
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-strict-harvest.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, buffer, production};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = (action(ev, "production-fired")
            && (production(ev, "attend-letter")
                || production(ev, "encode-letter")
                || production(ev, "respond")))
            || (action(ev, "clear-buffer")
                && ((ev.time_ms == 100 && buffer(ev, "visual-location"))
                    || (ev.time_ms == 235 && buffer(ev, "visual"))
                    || (ev.time_ms == 485 && buffer(ev, "imaginal"))));
        self.out.borrow_mut().record(line, keep, true);
    }
}

#[test]
fn hook_tutorial_unit2_demo2_strict_harvest() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit2-demo2-strict-harvest")
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
        .check_projection(Filter { out: out.clone() }, out, "\n", true)
}
