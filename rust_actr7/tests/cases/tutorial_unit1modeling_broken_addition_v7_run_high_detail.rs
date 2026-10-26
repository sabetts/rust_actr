//! case: tutorial-unit1modeling-broken-addition-v7-run-high-detail
//! ref:  tutorial-unit1modeling-broken-addition-v7-run-high-detail.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-broken-addition-v7-run-high-detail.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_broken_addition_v7_run_high_detail() -> Result<(), String> {
    Case::new("tutorial-unit1modeling-broken-addition-v7-run-high-detail")
        .models(&["test-models;models;broken-addition-v7.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::SetDetail(crate::Detail::High),
            Step::ModelOutput("? (run .3)".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.3,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
