//! case: tutorial-unit5modeling-v9-spreading-activation-only
//! ref:  tutorial-unit5modeling-v9-spreading-activation-only.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-spreading-activation-only.lisp

use crate::Case;

#[test]
// Full reason: the model and task are available, but
// printed-chunk-activation-trace is not implemented and recorded activation
// traces are empty; exact activation formatting and noise must be preserved.
#[ignore = "pending: activation-trace support + driver"]
fn tutorial_unit5modeling_v9_spreading_activation_only() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-spreading-activation-only")
        .seed((100, 0))
        .check()
}
