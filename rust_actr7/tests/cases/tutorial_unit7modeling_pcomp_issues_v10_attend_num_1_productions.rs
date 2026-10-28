//! case: tutorial-unit7modeling-pcomp-issues-v10-attend-num-1-productions
//! ref:  tutorial-unit7modeling-pcomp-issues-v10-attend-num-1-productions.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v10-attend-num-1-productions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_pcomp_issues_v10_attend_num_1_productions() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v10-attend-num-1-productions")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v10.lisp",
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
                name: "pp".into(),
                args: crate::sexps("PRODUCTION0"),
            },
        ])
        .check()
}
