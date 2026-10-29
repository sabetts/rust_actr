//! case: bst-learn
//! ref:  bst-learn.ref
//! lisp: actr7.x/test-models/bst-learn.lisp

use crate::Case;
use actr_engine::{BstMode, Step};

#[test]
fn bst_learn() -> Result<(), String> {
    Case::new("bst-learn")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::Sgp(":MODEL-WARNINGS".into(), crate::sexp("NIL")),
            Step::Sgp(":V".into(), crate::sexp("T")),
            Step::Bst(BstMode::Learn),
        ])
        .check()
}
