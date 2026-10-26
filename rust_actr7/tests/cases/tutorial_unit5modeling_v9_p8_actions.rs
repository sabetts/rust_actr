//! case: tutorial-unit5modeling-v9-p8-actions
//! ref:  tutorial-unit5modeling-v9-p8-actions.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-p8-actions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v9_p8_actions() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-p8-actions")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":NCNAR".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 27.48,
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
