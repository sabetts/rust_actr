//! case: zz-extended-motor-actions
//! ref:  zz-extended-motor-actions.ref
//! lisp: actr7.x/test-models/zz-extended-motor-actions.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn zz_extended_motor_actions() -> Result<(), String> {
    Case::new("zz-extended-motor-actions")
        .models(&["extras;extended-motor-actions;test-model.lisp"])
        .seed((100, 0))
        .steps(vec![Step::Reset, Step::ZzDriver])
        .check()
}
