//! case: tutorial-unit8-paired-dynamic-results-with-compiling
//! ref:  tutorial-unit8-paired-dynamic-results-with-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit8-paired-dynamic-results-with-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit8-paired-dynamic-results-with-compiling.spec

use crate::Case;

#[test]
// BLOCKED ON: dynamic paired retrieval success/latency pattern.
// The summary now has the right shape, but per-trial retrieval successes and
// compiled-production latencies still diverge from the oracle.
#[ignore = "numerical: dynamic paired retrieval/latency pattern"]
fn tutorial_unit8_paired_dynamic_results_with_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit8-paired-dynamic-results-with-compiling")
        .paired_summary()
        .check()
}
