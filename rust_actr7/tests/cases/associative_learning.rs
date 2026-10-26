//! case: associative-learning
//! ref:  associative-learning.ref
//! lisp: actr7.x/test-models/associative-learning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn associative_learning() -> Result<(), String> {
    Case::new("associative-learning")
        .models(&["extras;associative-learning;al-fan-study.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Reset, Step::AlDriver])
        .check()
}
