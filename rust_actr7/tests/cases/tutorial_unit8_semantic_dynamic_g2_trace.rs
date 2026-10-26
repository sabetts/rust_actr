//! case: tutorial-unit8-semantic-dynamic-g2-trace
//! ref:  tutorial-unit8-semantic-dynamic-g2-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit8-semantic-dynamic-g2-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_semantic_dynamic_g2_trace() -> Result<(), String> {
    Case::new("tutorial-unit8-semantic-dynamic-g2-trace")
        .models(&["tutorial;unit8;semantic-dynamic-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::GoalFocus(crate::chunk_desc(r#"G2"#)),
            Step::ModelOutput("? (goal-focus g2)\nG2".into()),
            Step::ModelOutput("? (run 1)".into()),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("0.75\n84\nNIL".into()),
            Step::GoalFocus(crate::chunk_desc(r#"G2"#)),
            Step::ModelOutput("? (goal-focus g2)\nG2".into()),
            Step::ModelOutput("? (run 1)".into()),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
