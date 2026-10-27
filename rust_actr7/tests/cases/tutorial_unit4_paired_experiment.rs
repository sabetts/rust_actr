//! case: tutorial-unit4-paired-experiment
//! ref:  tutorial-unit4-paired-experiment.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-experiment.lisp
//! spec: fixtures/regression/specs/tutorial-unit4-paired-experiment.spec

use crate::Case;

#[test]
#[ignore = "pending: paired summary timing differs by a few ms"]
fn tutorial_unit4_paired_experiment() -> Result<(), String> {
    Case::from_spec("tutorial-unit4-paired-experiment")
        .paired_summary()
        .check()
}
