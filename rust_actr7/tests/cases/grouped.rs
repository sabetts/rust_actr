//! case: grouped
//! ref:  grouped.ref
//! lisp: actr7.x/test-models/grouped.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn grouped() -> Result<(), String> {
    Case::new("grouped")
        .models(&[
            "tutorial;lisp;grouped.lisp",
            "tutorial;unit5;grouped-model.lisp",
        ])
        .seed((77093543977, 39813))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 20.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
