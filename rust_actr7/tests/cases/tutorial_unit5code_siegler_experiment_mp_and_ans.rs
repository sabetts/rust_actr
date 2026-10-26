//! case: tutorial-unit5code-siegler-experiment-mp-and-ans
//! ref:  tutorial-unit5code-siegler-experiment-mp-and-ans.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-mp-and-ans.lisp
//! spec: fixtures/regression/specs/tutorial-unit5code-siegler-experiment-mp-and-ans.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5code_siegler_experiment_mp_and_ans() -> Result<(), String> {
    Case::from_spec("tutorial-unit5code-siegler-experiment-mp-and-ans").check()
}
