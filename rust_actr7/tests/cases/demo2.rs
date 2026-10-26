//! case: demo2
//! ref:  demo2.ref
//! lisp: actr7.x/test-models/demo2.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn demo2() -> Result<(), String> {
    Case::new("demo2")
        .models(&[
            "tutorial;lisp;demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
