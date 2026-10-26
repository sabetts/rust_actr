//! case: tutorial-unit8-simple-ppm-model-initial-run
//! ref:  tutorial-unit8-simple-ppm-model-initial-run.ref
//! lisp: actr7.x/test-models/tutorial-unit8-simple-ppm-model-initial-run.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_simple_ppm_model_initial_run() -> Result<(), String> {
    Case::new("tutorial-unit8-simple-ppm-model-initial-run")
        .models(&["tutorial;unit8;simple-ppm-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 1.0,
            trace: true,
            width: 22,
        }])
        .check()
}
