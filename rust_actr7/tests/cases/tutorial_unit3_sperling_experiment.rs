//! case: tutorial-unit3-sperling-experiment
//! ref:  tutorial-unit3-sperling-experiment.ref
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-experiment.lisp
//! spec: fixtures/regression/specs/tutorial-unit3-sperling-experiment.spec

use crate::Case;

#[test]
// BLOCKED ON: exact Sperling experiment summary.
// Trial RNG/visicon ordering still shifts the reported proportions (#168).
#[ignore = "numerical: Sperling experiment summary"]
fn tutorial_unit3_sperling_experiment() -> Result<(), String> {
    Case::from_spec("tutorial-unit3-sperling-experiment").check()
}
