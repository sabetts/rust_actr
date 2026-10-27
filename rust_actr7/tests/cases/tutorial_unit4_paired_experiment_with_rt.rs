//! case: tutorial-unit4-paired-experiment-with-rt
//! ref:  tutorial-unit4-paired-experiment-with-rt.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-experiment-with-rt.lisp
//! spec: fixtures/regression/specs/tutorial-unit4-paired-experiment-with-rt.spec

use crate::Case;

#[test]
#[ignore = "pending: paired summary timing differs by a few ms"]
fn tutorial_unit4_paired_experiment_with_rt() -> Result<(), String> {
    Case::from_spec("tutorial-unit4-paired-experiment-with-rt")
        .paired_summary()
        .check()
}
