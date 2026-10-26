//! case: dynamic-slot-compilation
//! ref:  dynamic-slot-compilation.ref
//! lisp: actr7.x/test-models/dynamic-slot-compilation.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn dynamic_slot_compilation() -> Result<(), String> {
    Case::new("dynamic-slot-compilation")
        .models(&["test-models;models;dynamic-slot-compilation.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
