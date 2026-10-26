//! case: tutorial-unit5modeling-v1-simulated-retrieval
//! ref:  tutorial-unit5modeling-v1-simulated-retrieval.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v1-simulated-retrieval.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5modeling_v1_simulated_retrieval() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v1-simulated-retrieval")
        .models(&["test-models;models;declarative-issues-v1.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 2.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":RT".into(), crate::sexp(r#"-10"#)),
            Step::Seed((100, 3)),
        ])
        .check()
}
