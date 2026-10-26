//! case: unit-1-together-1-mp
//! ref:  unit-1-together-1-mp.ref
//! lisp: actr7.x/test-models/unit-1-together-1-mp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn unit_1_together_1_mp() -> Result<(), String> {
    Case::new("unit-1-together-1-mp")
        .models(&["examples;multiple-models;unit-1-together-1-mp.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: false,
            width: 22,
        }])
        .check()
}
