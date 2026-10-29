//! case: bst-learn-ppm-w-tree
//! ref:  bst-learn-ppm-w-tree.ref
//! lisp: actr7.x/test-models/bst-learn-ppm-w-tree.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn bst_learn_ppm_w_tree() -> Result<(), String> {
    Case::new("bst-learn-ppm-w-tree")
        .models(&["tutorial;lisp;bst-ppm.lisp", "tutorial;unit8;bst-ppm-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::Sgp(":STARTING-PARAMETERS".into(), crate::sexp(r#"(:SEED (1 0))"#)),
            Step::Reset,
            Step::Command { name: "chunk-type".into(), args: crate::sexps(r#"((CURSOR (:INCLUDE VISUAL-OBJECT)) (CURSOR T) SCREEN-POS VALUE STATUS COLOR HEIGHT WIDTH)"#) },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
            Step::Run { secs: 60.0, trace: true, width: 22 },
        ])
        .bst_summary()
        .check()
}
