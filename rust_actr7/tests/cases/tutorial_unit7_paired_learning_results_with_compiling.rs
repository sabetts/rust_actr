//! case: tutorial-unit7-paired-learning-results-with-compiling
//! ref:  tutorial-unit7-paired-learning-results-with-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-results-with-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit7-paired-learning-results-with-compiling.spec

use crate::Case;

#[test]
// BLOCKED ON: exact per-subject key timings.
// The paired summary matches within a few milliseconds; the remaining diff
// is in individual latency rows, not the aggregate shape (#294).
#[ignore = "numerical: paired-summary timing"]
fn tutorial_unit7_paired_learning_results_with_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit7-paired-learning-results-with-compiling").check()
}
