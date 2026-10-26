//! case: tutorial-unit8-simple-ppm-model-ppm-run
//! ref:  tutorial-unit8-simple-ppm-model-ppm-run.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-ppm-model-ppm-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_ppm_model_ppm_run() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-ppm-model-ppm-run")
        .models(&["tutorial;unit8;simple-ppm-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
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
