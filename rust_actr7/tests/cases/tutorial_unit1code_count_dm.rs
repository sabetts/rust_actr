//! case: tutorial-unit1code-count-dm
//! ref:  tutorial-unit1code-count-dm.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-count-dm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_count_dm() -> Result<(), String> {
    Case::new("tutorial-unit1code-count-dm")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (dm)".into()),
            Step::Command {
                name: "dm".into(),
                args: Vec::new(),
            },
            Step::ModelOutput("(FIVE FOUR THREE TWO ONE)".into()),
            Step::ModelOutput("".into()),
            Step::ModelOutput("? (dm four one)".into()),
            Step::Command {
                name: "dm".into(),
                args: crate::sexps(r#"(FOUR ONE)"#),
            },
            Step::ModelOutput("(FOUR ONE)".into()),
        ])
        .check()
}
