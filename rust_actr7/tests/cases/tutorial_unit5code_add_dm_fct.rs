//! case: tutorial-unit5code-add-dm-fct
//! ref:  tutorial-unit5code-add-dm-fct.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-add-dm-fct.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_add_dm_fct() -> Result<(), String> {
    Case::new("tutorial-unit5code-add-dm-fct")
        .models(&["tutorial;unit1;addition.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (add-dm-fct (list (list 'foo 'size *number*)))".into()),
            Step::Command {
                name: "add-dm-fct".into(),
                args: crate::sexps(r#"(((FOO SIZE 3)))"#),
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
