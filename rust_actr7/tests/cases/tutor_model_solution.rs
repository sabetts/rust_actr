//! case: tutor-model-solution
//! ref:  tutor-model-solution.ref
//! lisp: actr7.x/test-models/tutor-model-solution.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutor_model_solution() -> Result<(), String> {
    Case::new("tutor-model-solution")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
