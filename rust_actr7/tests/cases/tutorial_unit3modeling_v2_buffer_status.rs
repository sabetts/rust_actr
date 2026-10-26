//! case: tutorial-unit3modeling-v2-buffer-status
//! ref:  tutorial-unit3modeling-v2-buffer-status.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v2-buffer-status.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit3modeling_v2_buffer_status() -> Result<(), String> {
    Case::new("tutorial-unit3modeling-v2-buffer-status")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
            "test-models;models;perceptual-motor-issues-model-v2.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: false,
                width: 22,
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Command {
                name: "buffer-status".into(),
                args: crate::sexps(r#"(MANUAL)"#),
            },
        ])
        .check()
}
