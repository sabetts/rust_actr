//! case: test-mp-value-req-param
//! ref:  test-mp-value-req-param.ref
//! lisp: actr7.x/test-models/test-mp-value-req-param.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn test_mp_value_req_param() -> Result<(), String> {
    Case::new("test-mp-value-req-param")
        .models(&["test-models;models;test-mp-value-req-param.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":MP".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
