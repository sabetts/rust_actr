//! case: tutorial-unit5code-onehit-declare-buffer-usage-warning
//! lisp: actr7.x/test-models/tutorial-unit5code-onehit-declare-buffer-usage-warning.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5code_onehit_declare_buffer_usage_warning() -> Result<(), String> {
    Case::new("tutorial-unit5code-onehit-declare-buffer-usage-warning")
        .models(&[
            "test-models;models;tutorial-unit5code-onehit-declare-buffer-usage-warning-model.lisp",
        ])
        .steps(vec![Step::Reset])
        .check()
}
