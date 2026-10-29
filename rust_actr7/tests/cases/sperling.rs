//! case: sperling
//! ref:  sperling.ref
//! lisp: actr7.x/test-models/sperling.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn sperling() -> Result<(), String> {
    Case::new("sperling")
        .models(&[
            "tutorial;lisp;sperling.lisp",
            "tutorial;unit3;sperling-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(23),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::SetDetail(crate::Detail::High),
            Step::ConsumeRng(23),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
