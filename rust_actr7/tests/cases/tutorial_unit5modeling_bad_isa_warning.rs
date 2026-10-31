//! case: tutorial-unit5modeling-bad-isa-warning
//! ref:  tutorial-unit5modeling-bad-isa-warning.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-bad-isa-warning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_bad_isa_warning() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-bad-isa-warning")
        .models(&["tutorial;unit5;declarative-issues.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "delete-production".into(),
                args: vec![actr_engine::sexpr::Sexp::Sym("P1".into())],
            },
            Step::Command {
                name: "p".into(),
                args: crate::sexps(r#"P1 ==> +RETRIEVAL> ISA SIMPLE-VALUE ARG1 NIL"#),
            },
        ])
        .check()
}
