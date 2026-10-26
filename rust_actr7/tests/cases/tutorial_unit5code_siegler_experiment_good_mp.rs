//! case: tutorial-unit5code-siegler-experiment-good-mp
//! ref:  tutorial-unit5code-siegler-experiment-good-mp.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-good-mp.lisp
//! spec: fixtures/regression/specs/tutorial-unit5code-siegler-experiment-good-mp.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5code_siegler_experiment_good_mp() -> Result<(), String> {
    Case::from_spec("tutorial-unit5code-siegler-experiment-good-mp").check()
}
