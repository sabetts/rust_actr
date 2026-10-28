//! case: tutorial-unit7modeling-pcomp-issues-v11-test-runs
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v11-test-runs.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
#[ignore = "engine-divergence: pcomp multi-game compilation"]
fn tutorial_unit7modeling_pcomp_issues_v11_test_runs() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v11-test-runs")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "test-models;models;production-compilation-issues-model-v11.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::PcompGame(PcompGameConfig {
            games: 10,
            first_seed: None,
            epl: None,
            pct: None,
            show_games: true,
            continue_rng: true,
            print_seed: true,
            print_summary: true,
            stop_on_command: None,
        })])
        .check()
}
