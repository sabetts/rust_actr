//! case: tutorial-unit1-semantic-g3-trace
//! ref:  tutorial-unit1-semantic-g3-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit1-semantic-g3-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1_semantic_g3_trace() -> Result<(), String> {
    Case::new("tutorial-unit1-semantic-g3-trace")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::GoalFocus(crate::chunk_desc(
                r#"(ISA IS-MEMBER OBJECT CANARY CATEGORY FISH)"#,
            )),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::RunNEvents {
                count: 2,
                trace: false,
                width: 22,
            },
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
