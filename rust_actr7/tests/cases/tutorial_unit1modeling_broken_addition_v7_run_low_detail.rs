//! case: tutorial-unit1modeling-broken-addition-v7-run-low-detail
//! ref:  tutorial-unit1modeling-broken-addition-v7-run-low-detail.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v7-run-low-detail.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v7_run_low_detail() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v7-run-low-detail")
        .models(&["test-models;models;broken-addition-v7.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::SetDetail(crate::Detail::Low),
            Step::ModelOutput("? (run 1)".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
