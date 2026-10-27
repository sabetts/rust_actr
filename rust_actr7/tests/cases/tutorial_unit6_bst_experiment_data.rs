//! case: tutorial-unit6-bst-experiment-data
//! ref:  tutorial-unit6-bst-experiment-data.ref
//! lisp: actr7.x/test-models/tutorial-unit6-bst-experiment-data.lisp

use crate::Case;

#[test]
// Full reason: BST task/model sources are available, but this needs a live
// button-driven bst-set loop and production-utility averaging; the existing
// bst-learn cases still diverge.
#[ignore = "pending: BST experiment driver (sources available; divergence risk)"]
fn tutorial_unit6_bst_experiment_data() -> Result<(), String> {
    Case::new("tutorial-unit6-bst-experiment-data")
        .seed((100, 0))
        .check()
}
