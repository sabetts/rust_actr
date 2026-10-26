//! case: tutorial-unit8-paired-dynamic-compiling-to-dynamic
//! ref:  tutorial-unit8-paired-dynamic-compiling-to-dynamic.ref
//! lisp: actr7.x/test-models/tutorial-unit8-paired-dynamic-compiling-to-dynamic.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook system-parameter effects pending"]
fn tutorial_unit8_paired_dynamic_compiling_to_dynamic() -> Result<(), String> {
    Case::new("tutorial-unit8-paired-dynamic-compiling-to-dynamic")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
            "tutorial;unit8;paired-dynamic-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
