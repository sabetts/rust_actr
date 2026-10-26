//! case: tutorial-unit3-sperling-stuff-visual-location
//! ref:  tutorial-unit3-sperling-stuff-visual-location.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-stuff-visual-location.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, buffer};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
    started: bool,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let keep = !self.started && action(ev, "set-buffer-chunk") && buffer(ev, "visual-location");
        if keep {
            self.started = true;
        }
        self.out.borrow_mut().record(line, keep, false);
    }
}

#[test]
fn tutorial_unit3_sperling_stuff_visual_location() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit3-sperling-stuff-visual-location")
        .models(&["tutorial;lisp;sperling.lisp", "tutorial;unit3;sperling-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command { name: "permute-list".into(), args: crate::sexps(r#"("B" "C" "D" "F" "G" "H" "J" "K" "L" "M" "N" "P" "Q" "R" "S" "T" "V" "W" "X" "Y" "Z")"#) },
            Step::Command { name: "new-tone-sound".into(), args: crate::sexps(r#"(1000 0.5 0.15)"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(23),
            Step::Run { secs: 30.0, trace: true, width: 22 },
            Step::ModelOutput("".into()),
        ])
        .check_projection(Filter { out: out.clone(), started: false }, out, "\n", false)
}
