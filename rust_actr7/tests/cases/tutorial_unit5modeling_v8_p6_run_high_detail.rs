//! case: tutorial-unit5modeling-v8-p6-run-high-detail
//! ref:  tutorial-unit5modeling-v8-p6-run-high-detail.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v8-p6-run-high-detail.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v8_p6_run_high_detail() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v8-p6-run-high-detail")
        .models(&["test-models;models;declarative-issues-v8.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 27.15,
                trace: false,
                width: 22,
            },
            Step::SetDetail(crate::Detail::High),
            Step::Run {
                secs: 0.01,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
