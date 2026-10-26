//! case: tutorial-unit1code-run-semantic
//! ref:  tutorial-unit1code-run-semantic.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-run-semantic.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_run_semantic() -> Result<(), String> {
    Case::new("tutorial-unit1code-run-semantic")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (run 1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("0.15".into()),
            Step::ModelOutput("24".into()),
            Step::ModelOutput("NIL".into()),
        ])
        .check()
}
