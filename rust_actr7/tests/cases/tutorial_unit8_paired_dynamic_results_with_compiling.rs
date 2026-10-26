//! case: tutorial-unit8-paired-dynamic-results-with-compiling
//! ref:  tutorial-unit8-paired-dynamic-results-with-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit8-paired-dynamic-results-with-compiling.lisp
//! spec: fixtures/regression/specs/tutorial-unit8-paired-dynamic-results-with-compiling.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit8_paired_dynamic_results_with_compiling() -> Result<(), String> {
    Case::from_spec("tutorial-unit8-paired-dynamic-results-with-compiling").check()
}
