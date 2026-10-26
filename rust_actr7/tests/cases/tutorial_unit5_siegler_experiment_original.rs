//! case: tutorial-unit5-siegler-experiment-original
//! ref:  tutorial-unit5-siegler-experiment-original.ref
//! lisp: actr7.x/test-models/tutorial-unit5-siegler-experiment-original.lisp
//! spec: fixtures/regression/specs/tutorial-unit5-siegler-experiment-original.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5_siegler_experiment_original() -> Result<(), String> {
    Case::from_spec("tutorial-unit5-siegler-experiment-original").check()
}
