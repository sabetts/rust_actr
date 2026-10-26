//! case: tutorial-unit5modeling-v6-p5-run-with-mp-2
//! ref:  tutorial-unit5modeling-v6-p5-run-with-mp-2.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v6-p5-run-with-mp-2.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v6_p5_run_with_mp_2() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v6-p5-run-with-mp-2")
        .models(&["test-models;models;declarative-issues-v6.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":MP".into(), crate::sexp(r#"2"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 26.9,
                trace: false,
                width: 22,
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"MEDIUM"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 0.05,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
