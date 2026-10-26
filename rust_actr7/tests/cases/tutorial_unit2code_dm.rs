//! case: tutorial_unit2code_dm
//! ref:  tutorial_unit2code_dm.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_dm.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_dm() -> Result<(), String> {
    Case::new("tutorial_unit2code_dm")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput(">>> actr.dm()".into()),
            Step::Command {
                name: "dm".into(),
                args: Vec::new(),
            },
            Step::ModelOutput(
                "['TEN', 'NINE', 'EIGHT', 'SEVEN', 'SIX', 'FIVE', 'FOUR', 'THREE', 'TWO', 'ONE', 'ZERO']"
                    .into(),
            ),
            Step::ModelOutput(">>> actr.dm('one','three')".into()),
            Step::Command {
                name: "dm".into(),
                args: crate::sexps(r#"(ONE THREE)"#),
            },
            Step::ModelOutput("['ONE', 'THREE']".into()),
        ])
        .check()
}
