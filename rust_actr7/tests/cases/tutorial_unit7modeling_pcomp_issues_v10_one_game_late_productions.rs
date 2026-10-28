//! case: tutorial-unit7modeling-pcomp-issues-v10-one-game-late-productions
//! ref:  tutorial-unit7modeling-pcomp-issues-v10-one-game-late-productions.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v10-one-game-late-productions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7modeling_pcomp_issues_v10_one_game_late_productions() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v10-one-game-late-productions")
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
                args: crate::sexps("PRODUCTION775 PRODUCTION785"),
            },
        ])
        .check()
}
