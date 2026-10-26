//! case: test-temporal
//! ref:  test-temporal.ref
//! lisp: actr7.x/test-models/test-temporal.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn test_temporal() -> Result<(), String> {
    Case::new("test-temporal")
        .models(&["test-models;models;test-temporal.lisp"])
        .seed((50, 1))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 8.1,
                trace: true,
                width: 22,
            },
            Step::Sgp(":RECORD-TICKS".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
