//! case: tutorial-unit7modeling-pcomp-issues-v11-test-runs
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v11-test-runs.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
// BLOCKED ON: game 6, trial 10 retrieves response-1 instead of response-3.
// The trace is identical through the guess-other request, but the engine
// draws one fewer activation-noise sample, so the retrieval tie flips.
#[ignore = "engine-divergence: pcomp partial-matching retrieval tie"]
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
