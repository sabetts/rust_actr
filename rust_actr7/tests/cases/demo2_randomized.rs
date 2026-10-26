//! case: demo2-randomized
//! ref:  demo2-randomized.ref
//! lisp: actr7.x/test-models/demo2-randomized.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn demo2_randomized() -> Result<(), String> {
    Case::new("demo2-randomized")
        .models(&[
            "tutorial;lisp;demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .seed((123456, 0))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":RANDOMIZE-TIME".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":VPFT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":VIDT".into(), crate::sexp(r#"T"#)),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
