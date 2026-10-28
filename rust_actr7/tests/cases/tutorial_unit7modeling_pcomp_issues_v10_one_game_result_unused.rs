//! case: tutorial-unit7modeling-pcomp-issues-v10-one-game-result-unused
//! lisp: actr7.x/test-models/tutorial-unit7modeling-pcomp-issues-v10-one-game-result-unused.lisp

use crate::Case;
use actr_engine::{PcompGameConfig, Step};

#[test]
fn tutorial_unit7modeling_pcomp_issues_v10_one_game_result_unused() -> Result<(), String> {
    Case::new("tutorial-unit7modeling-pcomp-issues-v10-one-game-result-unused")
        .models(&[
            "tutorial;lisp;pcomp-issues.lisp",
            "test-models;models;production-compilation-issues-model-v10.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::PcompGame(PcompGameConfig {
                games: 1,
                first_seed: None,
                epl: None,
                pct: None,
                show_games: false,
                continue_rng: false,
                print_seed: false,
                print_summary: false,
                stop_on_command: None,
            }),
            Step::PcompUtilityNames,
        ])
        .check()
}
