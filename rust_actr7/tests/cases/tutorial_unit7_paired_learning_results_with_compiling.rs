//! case: tutorial-unit7-paired-learning-results-with-compiling
//! ref:  tutorial-unit7-paired-learning-results-with-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-results-with-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit7-paired-learning-results-with-compiling.spec

use crate::Case;

#[test]
fn tutorial_unit7_paired_learning_results_with_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit7-paired-learning-results-with-compiling")
        .paired_summary()
        .check()
}
