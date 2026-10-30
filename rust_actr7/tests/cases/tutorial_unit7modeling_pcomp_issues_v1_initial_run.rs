//! case: tutorial-unit7modeling-pcomp-issues-v1-initial-run
//! ref:  tutorial-unit7modeling-pcomp-issues-v1-initial-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v1-initial-run.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
fn tutorial_unit7modeling_pcomp_issues_v1_initial_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v1-initial-run")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v1.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::PcompGame(PcompGameConfig {
            games: 50,
            first_seed: Some(101),
            epl: Some(true),
            pct: None,
            show_games: false,
            continue_rng: false,
            print_seed: false,
            print_summary: true,
            stop_on_command: None,
        })])
        .check()
}
