//! case: tutorial-unit4-zbrodoff-with-retrieval
//! ref:  tutorial-unit4-zbrodoff-with-retrieval.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-with-retrieval.lisp

use crate::Case;

#[test]
// Full reason: this task loads
// examples/act-r-solutions/unit-4-zbrodoff-solution-model.lisp from the
// password-protected act-r-solutions.zip; the password is not in the repo or
// git history.
#[ignore = "blocked: protected Zbrodoff solution model"]
fn tutorial_unit4_zbrodoff_with_retrieval() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-with-retrieval")
        .seed((100, 0))
        .check()
}
