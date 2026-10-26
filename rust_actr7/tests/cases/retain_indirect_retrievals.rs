//! case: retain-indirect-retrievals
//! ref:  retain-indirect-retrievals.ref
//! lisp: actr7.x/test-models/retain-indirect-retrievals.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn retain_indirect_retrievals() -> Result<(), String> {
    Case::new("retain-indirect-retrievals")
        .models(&["test-models;models;retain-indirect-retrievals.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":CIA".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":CIA".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":CIA".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":RIR".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
