//! case: tutorial-unit5modeling-v8-p6-run
//! ref:  tutorial-unit5modeling-v8-p6-run.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v8-p6-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v8_p6_run() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v8-p6-run")
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
            Step::Sgp(":ACT".into(), crate::sexp(r#"MEDIUM"#)),
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunUntilTime {
                time: 28.422,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
