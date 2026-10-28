//! case: tutorial-unit7modeling-pcomp-issues-v5-initial-run
//! ref:  tutorial-unit7modeling-pcomp-issues-v5-initial-run.ref
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v5-initial-run.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
fn tutorial_unit7modeling_pcomp_issues_v5_initial_run() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v5-initial-run")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "tutorial;unit7;production-compilation-issues-model.lisp",
            "test-models;models;production-compilation-issues-model-v5.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::PcompGame(PcompGameConfig {
            games: 10,
            first_seed: Some(301),
            epl: Some(false),
            pct: Some(true),
            show_games: false,
            continue_rng: false,
            print_seed: false,
            print_summary: true,
        })])
        .check()
}
