//! case: tutorial-unit8-simple-ppm-model-medium-chosen-run
//! ref:  tutorial-unit8-simple-ppm-model-medium-chosen-run.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-ppm-model-medium-chosen-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_ppm_model_medium_chosen_run() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-ppm-model-medium-chosen-run")
        .models(&["tutorial;unit8;simple-ppm-model.lisp"])
        .seed((150264085143, 2))
        .steps(vec![
            Step::Reset,
            Step::Sgp(":PPM".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":EGS".into(), crate::sexp(r#"1"#)),
            Step::Sgp(":UTILITY-OFFSETS".into(), crate::sexp(r#"PPM-OFFSET"#)),
            Step::Seed((150264085143, 2)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
