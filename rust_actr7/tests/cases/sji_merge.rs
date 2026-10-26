//! case: sji-merge
//! ref:  sji-merge.ref
//! lisp: actr7.x/test-models/sji-merge.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn sji_merge() -> Result<(), String> {
    Case::new("sji-merge")
        .models(&["test-models;models;sji-merge.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Run {
            secs: 10.0,
            trace: true,
            width: 22,
        }])
        .check()
}
