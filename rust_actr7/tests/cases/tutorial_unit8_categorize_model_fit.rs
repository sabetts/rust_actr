//! case: tutorial-unit8-categorize-model-fit
//! ref:  tutorial-unit8-categorize-model-fit.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-model-fit.lisp

use crate::Case;

#[test]
// Full reason: this task loads
// examples/act-r-solutions/unit-8-categorize-solution-model.lisp from the
// password-protected act-r-solutions.zip; the password is not in the repo or
// git history.
#[ignore = "blocked: protected categorize solution model"]
fn tutorial_unit8_categorize_model_fit() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-model-fit")
        .seed((100, 0))
        .check()
}
