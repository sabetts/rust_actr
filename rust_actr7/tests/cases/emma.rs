//! case: emma
//! ref:  emma.ref
//! lisp: actr7.x/test-models/emma.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn emma() -> Result<(), String> {
    Case::new("emma")
        .models(&[
            "extras;emma;example-demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":STARTING-PARAMETERS".into(), crate::sexp(r#"(:EMMA T)"#)),
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Sgp(":STARTING-PARAMETERS".into(), crate::sexp(r#"NIL"#)),
        ])
        .check()
}
