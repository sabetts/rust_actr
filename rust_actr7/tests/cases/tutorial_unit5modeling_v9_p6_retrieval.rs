//! case: tutorial-unit5modeling-v9-p6-retrieval
//! ref:  tutorial-unit5modeling-v9-p6-retrieval.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-p6-retrieval.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v9_p6_retrieval() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-p6-retrieval")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
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
            Step::RunUntilTime {
                time: 27.381,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
