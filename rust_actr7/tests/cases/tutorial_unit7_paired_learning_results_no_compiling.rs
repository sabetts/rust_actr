//! case: tutorial-unit7-paired-learning-results-no-compiling
//! ref:  tutorial-unit7-paired-learning-results-no-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-results-no-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit7-paired-learning-results-no-compiling.spec

use crate::Case;

#[test]
fn tutorial_unit7_paired_learning_results_no_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit7-paired-learning-results-no-compiling")
        .paired_summary()
        .check()
}
