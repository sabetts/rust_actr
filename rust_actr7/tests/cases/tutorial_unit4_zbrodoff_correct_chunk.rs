//! case: tutorial-unit4-zbrodoff-correct-chunk
//! ref:  tutorial-unit4-zbrodoff-correct-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-correct-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit4_zbrodoff_correct_chunk() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-correct-chunk")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::ZbrodoffProblem {
                addend1: "a".into(),
                addend2: "2".into(),
                sum: "c".into(),
                answer: "k".into(),
            },
            Step::Sdm(vec!["RESULT".into(), "C".into()]),
        ])
        .check()
}
