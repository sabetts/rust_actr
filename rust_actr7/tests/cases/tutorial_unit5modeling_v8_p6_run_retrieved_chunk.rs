//! case: tutorial-unit5modeling-v8-p6-run-retrieved-chunk
//! ref:  tutorial-unit5modeling-v8-p6-run-retrieved-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v8-p6-run-retrieved-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v8_p6_run_retrieved_chunk() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v8-p6-run-retrieved-chunk")
        .models(&["test-models;models;declarative-issues-v8.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 27.155,
                trace: false,
                width: 22,
            },
            Step::RunUntilTime {
                time: 28.421,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("".into()),
            Step::Command {
                name: "pprint-chunks".into(),
                args: crate::sexps(r#"(1-1=0)"#),
            },
        ])
        .check()
}
