//! case: tutorial-unit6-choice-results
//! ref:  tutorial-unit6-choice-results.ref
//! lisp: actr7.x/test-models/tutorial-unit6-choice-results.lisp

use crate::Case;

#[test]
// Full reason: this task loads
// examples/act-r-solutions/unit-6-choice-solution-task.lisp from the
// password-protected act-r-solutions.zip; the password is not in the repo or
// git history.
#[ignore = "blocked: protected choice task/model source"]
fn tutorial_unit6_choice_results() -> Result<(), String> {
    Case::new("tutorial-unit6-choice-results")
        .seed((100, 0))
        .check()
}
