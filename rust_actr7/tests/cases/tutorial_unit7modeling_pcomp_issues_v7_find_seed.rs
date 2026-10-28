//! case: tutorial-unit7modeling-pcomp-issues-v7-find-seed
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v7-find-seed.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
fn tutorial_unit7modeling_pcomp_issues_v7_find_seed() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v7-find-seed")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "test-models;models;production-compilation-issues-model-v7.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::RegisterTaskCommand("set-flag".to_string()),
            Step::PcompGame(PcompGameConfig {
                games: 100,
                first_seed: None,
                epl: None,
                pct: None,
                show_games: false,
                continue_rng: true,
                print_seed: true,
                print_summary: true,
                stop_on_command: Some("set-flag"),
            }),
        ])
        .check()
}
