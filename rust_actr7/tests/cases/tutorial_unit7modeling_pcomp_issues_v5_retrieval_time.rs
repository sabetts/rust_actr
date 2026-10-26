//! case: tutorial-unit7modeling-pcomp-issues-v5-retrieval-time
//! ref:  tutorial-unit7modeling-pcomp-issues-v5-retrieval-time.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v5-retrieval-time.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook conversion pending"]
fn tutorial_unit7modeling_pcomp_issues_v5_retrieval_time() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v5-retrieval-time")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v5.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: true,
                width: 22,
            },
            Step::RunNEvents {
                count: 13,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
