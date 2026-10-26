//! case: tutorial-unit3modeling-v4-previous-segment-trace
//! ref:  tutorial-unit3modeling-v4-previous-segment-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v4-previous-segment-trace.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = ev.time_ms >= 2285
            && (ev.time_ms < 2420 || (ev.time_ms == 2420 && action(ev, "production-fired")));
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn tutorial_unit3modeling_v4_previous_segment_trace() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit3modeling-v4-previous-segment-trace")
        .models(&["tutorial;lisp;pm-issues.lisp", "tutorial;unit3;perceptual-motor-issues-model.lisp", "test-models;models;perceptual-motor-issues-model-v4.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "permute-list".into(), args: crate::sexps(r#"("a" "b" "c" "d" "e" "f" "g" "h" "i" "j" "k" "l" "m" "n" "o" "p" "q" "r" "s" "t" "u" "v" "w" "x" "y" "z")"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(27),
            Step::Run { secs: 10.0, trace: true, width: 22 },
            Step::ModelOutput("".into()),
        ])
        .check_projection(Filter { out: out.clone() }, out, "\n", false)
}
