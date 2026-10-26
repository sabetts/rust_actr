//! case: tutorial-unit3-sperling-do-report-trace
//! ref:  tutorial-unit3-sperling-do-report-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-do-report-trace.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, production};
use actr_engine::{EventHook, EventView, Step};

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Idle,
    Collect,
    Done,
}

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
    phase: Phase,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        let collect = self.phase == Phase::Collect
            || (self.phase == Phase::Idle
                && action(ev, "production-fired")
                && production(ev, "do-report"));
        if collect {
            self.out.borrow_mut().push_line(line);
            if self.phase == Phase::Idle {
                self.phase = Phase::Collect;
            } else if self.phase == Phase::Collect && action(ev, "production-fired") {
                self.phase = Phase::Done;
            }
        } else {
            self.out.borrow_mut().push_ellipsis();
        }
    }
}

#[test]
fn tutorial_unit3_sperling_do_report_trace() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit3-sperling-do-report-trace")
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
        .check_projection(Filter { out: out.clone(), phase: Phase::Idle }, out, "\n", false)
}
