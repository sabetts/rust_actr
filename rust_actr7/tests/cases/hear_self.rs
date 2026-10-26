//! case: hear-self
//! ref:  hear-self.ref
//! lisp: actr7.x/test-models/hear-self.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn hear_self() -> Result<(), String> {
    Case::new("hear-self")
        .models(&["test-models;models;hear-self.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":HEAR-SELF".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":HEAR-SELF".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":HEAR-SELF".into(), crate::sexp(r#"SPEAK"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":HEAR-SELF".into(), crate::sexp(r#"SUBVOCALIZE"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
