//! case: unit-1-together-1-mp
//! ref:  unit-1-together-1-mp.ref
//! lisp: actr7.x/test-models/unit-1-together-1-mp.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: multi-model meta-process trace.
// The ref interleaves COUNT/SEMANTIC/ADDITION and adds a model-name column;
// the engine still loads this as a single-model trace (#66).
#[ignore = "missing-feature: multi-model trace columns"]
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
