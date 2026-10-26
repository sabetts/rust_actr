//! case: save-model
//! ref:  save-model.ref
//! lisp: actr7.x/test-models/save-model.lisp

use crate::Case;

#[test]
#[ignore = "engine ops not in driver: demo2-experiment run + save-model-file hooks (pending)"]
fn save_model() -> Result<(), String> {
    Case::new("save-model").seed((100, 0)).check()
}
