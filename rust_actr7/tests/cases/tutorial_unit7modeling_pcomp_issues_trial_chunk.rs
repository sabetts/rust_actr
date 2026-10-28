//! case: tutorial-unit7modeling-pcomp-issues-trial-chunk
//! ref:  tutorial-unit7modeling-pcomp-issues-trial-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-trial-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_pcomp_issues_trial_chunk() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-trial-chunk")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Seed((100, 1)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["IMAGINAL".into()]),
        ])
        .check()
}
