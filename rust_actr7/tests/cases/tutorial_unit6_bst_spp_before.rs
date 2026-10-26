//! case: tutorial-unit6-bst-spp-before
//! ref:  tutorial-unit6-bst-spp-before.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-spp-before.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit6_bst_spp_before() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-spp-before")
        .models(&["tutorial;lisp;bst.lisp", "tutorial;unit6;bst-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "spp".into(),
            args: crate::sexps(r#"(FORCE-OVER FORCE-UNDER DECIDE-OVER DECIDE-UNDER)"#),
        }])
        .check()
}
