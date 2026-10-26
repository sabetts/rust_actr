//! case: tutorial-unit5modeling-v5-p3-run-queue
//! ref:  tutorial-unit5modeling-v5-p3-run-queue.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v5-p3-run-queue.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook conversion pending"]
fn tutorial_unit5modeling_v5_p3_run_queue() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v5-p3-run-queue")
        .models(&["test-models;models;declarative-issues-v5.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
        ])
        .check()
}
