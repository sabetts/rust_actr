//! case: tutorial-unit7-past-tense-no-reward-warning
//! ref:  tutorial-unit7-past-tense-no-reward-warning.ref
//! lisp: actr7.x/test-models/tutorial-unit7-past-tense-no-reward-warning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit7_past_tense_no_reward_warning() -> Result<(), String> {
    Case::new("tutorial-unit7-past-tense-no-reward-warning")
        .models(&[
            "tutorial;lisp;past-tense.lisp",
            "tutorial;unit7;past-tense-model.lisp",
        ])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp(r#"NIL"#)),
            Step::PastTenseWarning {
                override_buffers: false,
            },
        ])
        .check()
}
