//! case: tutorial-unit5code-add-dm
//! ref:  tutorial-unit5code-add-dm.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-add-dm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_add_dm() -> Result<(), String> {
    Case::new("tutorial-unit5code-add-dm")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (defvar *number* 3)".into()),
            Step::ModelOutput("*NUMBER*".into()),
            Step::ModelOutput("? (add-dm (foo size *number*))".into()),
            Step::Command {
                name: "add-dm".into(),
                args: crate::sexps(r#"((FOO SIZE *NUMBER*))"#),
            },
            Step::ModelOutput("(FOO)".into()),
            Step::ModelOutput("? (dm foo)".into()),
            Step::Command {
                name: "dm".into(),
                args: crate::sexps(r#"(FOO)"#),
            },
        ])
        .check()
}
