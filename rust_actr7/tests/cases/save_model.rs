//! case: save-model
//! ref:  save-model.ref
//! lisp: actr7.x/test-models/save-model.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn save_model() -> Result<(), String> {
    Case::new("save-model")
        .models(&[
            "tutorial;lisp;demo2.lisp",
            "tutorial;unit2;demo2-model.lisp",
        ])
        .device("demo2")
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            // demo2's `permute-list` draw before the run.
            Step::ConsumeRng(21),
            Step::Run {
                secs: 10.0,
                trace: true,
                width: 22,
            },
            Step::ModelOutput("DEMO".into()),
            Step::SaveModel {
                timestamp: Some("2025/12/17 14:47:15".to_string()),
            },
        ])
        .check()
}
