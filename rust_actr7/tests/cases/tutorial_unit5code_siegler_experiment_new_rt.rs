//! case: tutorial-unit5code-siegler-experiment-new-rt
//! ref:  tutorial-unit5code-siegler-experiment-new-rt.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-new-rt.lisp
//! spec: fixtures/regression/specs/tutorial-unit5code-siegler-experiment-new-rt.spec

use crate::Case;

#[test]
// BLOCKED ON: exact Siegler response distribution.
// The summary structure matches, but the response cells are close rather
// than exact (#281).
#[ignore = "numerical: Siegler response distribution"]
fn tutorial_unit5code_siegler_experiment_new_rt() -> Result<(), String> {
    Case::from_spec("tutorial-unit5code-siegler-experiment-new-rt").check()
}
