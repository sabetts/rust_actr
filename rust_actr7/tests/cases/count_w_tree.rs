//! case: count-w-tree
//! ref:  count-w-tree.ref
//! lisp: actr7.x/test-models/count-w-tree.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn count_w_tree() -> Result<(), String> {
    Case::new("count-w-tree")
        .models(&["tutorial;unit1;count.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Command {
                name: "print-conflict-tree".into(),
                args: crate::sexps(r#"()"#),
            },
        ])
        .check()
}
