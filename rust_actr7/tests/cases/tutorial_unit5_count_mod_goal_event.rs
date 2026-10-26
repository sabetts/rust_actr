//! case: tutorial-unit5-count-mod-goal-event
//! ref:  tutorial-unit5-count-mod-goal-event.ref
//! lisp: actr7.x/test-models/tutorial-unit5-count-mod-goal-event.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = action(ev, "mod-buffer-chunk");
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn tutorial_unit5_count_mod_goal_event() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit5-count-mod-goal-event")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
        ])
        .check_projection(Filter { out: out.clone() }, out, "\n", false)
}
