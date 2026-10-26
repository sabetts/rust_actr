//! case: tutorial-unit5modeling-v8-chunk-1-plus-2
//! ref:  tutorial-unit5modeling-v8-chunk-1-plus-2.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v8-chunk-1-plus-2.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v8_chunk_1_plus_2() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v8-chunk-1-plus-2")
        .models(&["test-models;models;declarative-issues-v8.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Command {
            name: "pprint-chunks".into(),
            args: crate::sexps(r#"(1+2=3)"#),
        }])
        .check()
}
