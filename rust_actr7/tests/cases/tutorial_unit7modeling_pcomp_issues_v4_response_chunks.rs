//! case: tutorial-unit7modeling-pcomp-issues-v4-response-chunks
//! ref:  tutorial-unit7modeling-pcomp-issues-v4-response-chunks.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v4-response-chunks.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_pcomp_issues_v4_response_chunks() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v4-response-chunks")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v4.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::ResetSilent,
            Step::ConsumeRng(2),
            Step::Run {
                secs: 20000.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::Command {
                name: "pprint-chunks".into(),
                args: crate::sexps("RESPONSE-2-0 RESPONSE-2-1"),
            },
        ])
        .check()
}
