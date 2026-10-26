//! case: safe-queries
//! ref:  safe-queries.ref
//! lisp: actr7.x/test-models/safe-queries.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn safe_queries() -> Result<(), String> {
    Case::new("safe-queries")
        .models(&["test-models;models;safe-queries.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Command {
                name: "pp".into(),
                args: crate::sexps(r#"()"#),
            },
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(r#"(:DO-NOT-QUERY IMAGINAL :TESTING-FUNCTION SET-TESTING-TRACES)"#),
            ),
            Step::Reset,
            Step::Command {
                name: "pp".into(),
                args: crate::sexps(r#"()"#),
            },
        ])
        .check()
}
