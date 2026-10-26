//! case: tutorial-unit5code-siegler-experiment-rt-and-base-level
//! ref:  tutorial-unit5code-siegler-experiment-rt-and-base-level.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-rt-and-base-level.lisp
//! spec: fixtures/regression/specs/tutorial-unit5code-siegler-experiment-rt-and-base-level.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5code_siegler_experiment_rt_and_base_level() -> Result<(), String> {
    Case::from_spec("tutorial-unit5code-siegler-experiment-rt-and-base-level").check()
}
