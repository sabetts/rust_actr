//! case: tutorial-unit4-paired-experiment-only-bll
//! ref:  tutorial-unit4-paired-experiment-only-bll.ref
//! lisp: actr7.x/test-models/tutorial-unit4-paired-experiment-only-bll.lisp
//! spec: fixtures/regression/specs/tutorial-unit4-paired-experiment-only-bll.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit4_paired_experiment_only_bll() -> Result<(), String> {
    Case::from_spec("tutorial-unit4-paired-experiment-only-bll").check()
}
