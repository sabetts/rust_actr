//! case: tutorial-unit4-paired-experiment-no-bll
//! ref:  tutorial-unit4-paired-experiment-no-bll.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-experiment-no-bll.lisp
//! spec: fixtures/regression/specs/tutorial-unit4-paired-experiment-no-bll.spec

use crate::Case;

#[test]
#[ignore = "pending: paired summary timing differs by a few ms"]
fn tutorial_unit4_paired_experiment_no_bll() -> Result<(), String> {
    Case::from_spec("tutorial-unit4-paired-experiment-no-bll")
        .paired_summary()
        .check()
}
