//! case: tutorial-unit1modeling-broken-addition-v6-run
//! ref:  tutorial-unit1modeling-broken-addition-v6-run.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v6-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v6_run() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v6-run")
        .models(&["test-models;models;broken-addition-v6.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (run 10)".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
