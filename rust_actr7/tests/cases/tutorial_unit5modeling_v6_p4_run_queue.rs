//! case: tutorial-unit5modeling-v6-p4-run-queue
//! ref:  tutorial-unit5modeling-v6-p4-run-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v6-p4-run-queue.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit5modeling_v6_p4_run_queue() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v6-p4-run-queue")
        .models(&["test-models;models;declarative-issues-v6.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: false,
                width: 22,
            },
        ])
        .check()
}
