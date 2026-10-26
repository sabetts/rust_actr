//! case: tutorial-unit8-categorize-sample-goal-chunk
//! ref:  tutorial-unit8-categorize-sample-goal-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit8-categorize-sample-goal-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit8_categorize_sample_goal_chunk() -> Result<(), String> {
    Case::new("tutorial-unit8-categorize-sample-goal-chunk")
        .models(&[
            "tutorial;lisp;categorize.lisp",
            "tutorial;unit8;categorize-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::GoalFocus(crate::chunk_desc(
                "(name eh state add-attribute value 0.704)",
            )),
            Step::Run {
                secs: 20.0,
                trace: false,
                width: 22,
            },
            Step::BufferChunk(vec!["GOAL".into()]),
        ])
        .check()
}
