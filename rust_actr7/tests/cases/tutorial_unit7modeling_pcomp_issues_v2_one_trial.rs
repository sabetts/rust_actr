//! case: tutorial-unit7modeling-pcomp-issues-v2-one-trial
//! ref:  tutorial-unit7modeling-pcomp-issues-v2-one-trial.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v2-one-trial.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_pcomp_issues_v2_one_trial() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v2-one-trial")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v2.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 20000.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
