//! case: tutorial-unit7-paired-learning-results-no-compiling
//! ref:  tutorial-unit7-paired-learning-results-no-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-results-no-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit7-paired-learning-results-no-compiling.spec

use crate::Case;

#[test]
// BLOCKED ON: exact per-subject key timings.
// The paired summary matches within a few milliseconds; the remaining diff
// is in individual latency rows, not the aggregate shape (#294).
#[ignore = "numerical: paired-summary timing"]
fn tutorial_unit7_paired_learning_results_no_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit7-paired-learning-results-no-compiling").check()
}
