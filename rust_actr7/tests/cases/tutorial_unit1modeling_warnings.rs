//! case: tutorial-unit1modeling-warnings
//! ref:  tutorial-unit1modeling-warnings.ref
//! lisp: actr7.x/test-models/tutorial-unit1modeling-warnings.lisp

use crate::Case;
use actr_engine::sexpr::Sexp;
use actr_engine::Step;

#[test]
fn tutorial_unit1modeling_warnings() -> Result<(), String> {
    // The stub defines the model, requests retrieval twice, then redefines
    // TEST. The no-slots warning comes from `define-chunks`, the abort warning
    // from the second request, and the redefinition warning from `(p ...)`.
    Case::new("tutorial-unit1modeling-warnings")
        .models(&["test-models;models;tutorial-unit1modeling-warnings.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp("(:model-warnings t)"),
            ),
            Step::Reset,
            Step::EmptyModuleRequest("RETRIEVAL".into()),
            Step::EmptyModuleRequest("RETRIEVAL".into()),
            Step::Command {
                name: "p".into(),
                args: vec![Sexp::Sym("TEST".into()), Sexp::Sym("==>".into())],
            },
        ])
        .check()
}
