//! case: tutorial-unit5modeling-v3-first-run
//! ref:  tutorial-unit5modeling-v3-first-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v3-first-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v3_first_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v3-first-run")
        .models(&["test-models;models;declarative-issues-v3.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"LOW"#)),
            Step::Run {
                secs: 12.26,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
