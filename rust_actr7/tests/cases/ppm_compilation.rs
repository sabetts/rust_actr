//! case: ppm-compilation
//! ref:  ppm-compilation.ref
//! lisp: actr7.x/test-models/ppm-compilation.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn ppm_compilation() -> Result<(), String> {
    Case::new("ppm-compilation")
        .models(&["test-models;models;ppm-compilation.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
