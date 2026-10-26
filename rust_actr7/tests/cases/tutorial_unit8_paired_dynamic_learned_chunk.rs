//! case: tutorial-unit8-paired-dynamic-learned-chunk
//! ref:  tutorial-unit8-paired-dynamic-learned-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-paired-dynamic-learned-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_paired_dynamic_learned_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-paired-dynamic-learned-chunk")
        .models(&[
            "tutorial;lisp;paired.lisp",
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
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: false,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::Sgp(":V".into(), crate::sexp(r#"T"#)),
            Step::Sdm(vec!["-".into(), "WORD".into(), "NIL".into()]),
        ])
        .check()
}
