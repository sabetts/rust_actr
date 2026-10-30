//! case: tutorial-unit7modeling-pcomp-issues-v12-50-runs
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v12-50-runs.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
fn tutorial_unit7modeling_pcomp_issues_v12_50_runs() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v12-50-runs")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "test-models;models;production-compilation-issues-model-v12.lisp",
        ])
        .seed((100, 0))
        .steps(vec![Step::PcompGame(PcompGameConfig {
            games: 50,
            first_seed: None,
            epl: None,
            pct: None,
            show_games: false,
            continue_rng: true,
            print_seed: false,
            print_summary: true,
            stop_on_command: None,
        })])
        .check()
}
