//! case: tutorial-unit3-sperling-initial-audio-trace
//! ref:  tutorial-unit3-sperling-initial-audio-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-initial-audio-trace.lisp

use std::cell::RefCell;
use std::rc::Rc;

use crate::hooks::{self, action, module};
use actr_engine::{EventHook, EventView, Step};

struct Filter {
    out: Rc<RefCell<hooks::Projection>>,
}

impl EventHook for Filter {
    fn post(&mut self, line: &str, ev: &EventView) {
        if action(ev, "buffer-read-action") || action(ev, "query-buffer-action") {
            return;
        }
        let keep = (135..=236).contains(&ev.time_ms)
            && (module(ev, "AUDIO")
                || line.to_uppercase().contains("AUDIO")
                || (module(ev, "PROCEDURAL")
                    && (action(ev, "production-selected") || action(ev, "production-fired"))));
        if keep {
            self.out.borrow_mut().push_line(line);
        } else {
            self.out.borrow_mut().push_ellipsis();
        }
    }
}

#[test]
fn tutorial_unit3_sperling_initial_audio_trace() -> Result<(), String> {
    let out = Rc::new(RefCell::new(hooks::Projection::default()));
    hooks::HookCase::new("tutorial-unit3-sperling-initial-audio-trace")
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
            Step::SetDetail(crate::Detail::High),
            Step::Run { secs: 30.0, trace: true, width: 22 },
            Step::ModelOutput("".into()),
        ])
        .check_projection(Filter { out: out.clone() }, out, "\n", false)
}
