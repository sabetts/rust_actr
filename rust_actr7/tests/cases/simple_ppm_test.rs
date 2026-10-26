//! case: simple-ppm-test
//! ref:  simple-ppm-test.ref
//! lisp: actr7.x/test-models/simple-ppm-test.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn simple_ppm_test() -> Result<(), String> {
    Case::new("simple-ppm-test")
        .models(&["tutorial;unit8;simple-ppm-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Sgp(":PPM".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":UTILITY-OFFSETS".into(), crate::sexp(r#"PPM-OFFSET"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
            Step::Reset,
            Step::Seed((100, 1)),
            Step::Sgp(":EGS".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":PPM".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":UTILITY-OFFSETS".into(), crate::sexp(r#"PPM-OFFSET"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
