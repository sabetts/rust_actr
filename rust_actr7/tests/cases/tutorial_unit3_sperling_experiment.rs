//! case: tutorial-unit3-sperling-experiment
//! ref:  tutorial-unit3-sperling-experiment.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-experiment.lisp
//! spec: fixtures/regression/specs/tutorial-unit3-sperling-experiment.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit3_sperling_experiment() -> Result<(), String> {
    Case::from_spec("tutorial-unit3-sperling-experiment").check()
}
