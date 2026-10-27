//! case: tutorial-unit6-bst-spp-after-1-use
//! ref:  tutorial-unit6-bst-spp-after-1-use.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-spp-after-1-use.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit6_bst_spp_after_1_use() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-spp-after-1-use")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((1, 1))
        .steps(vec![
            Step::Sgp(
                ":STARTING-PARAMETERS".into(),
                crate::sexp(
                    r#"(:SEED (1 1) :MODEL-WARNINGS NIL :TESTING-FUNCTION SET-TESTING-TRACES)"#,
                ),
            ),
            Step::Reset,
            Step::Sgp(":V".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 60.0,
                trace: false,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":V".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 60.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "spp".into(),
                args: crate::sexps(r#"(FORCE-OVER FORCE-UNDER DECIDE-OVER DECIDE-UNDER)"#),
            },
        ])
        .check()
}
