//! case: tutorial-unit5-onehit-lisp-learning
//! ref:  tutorial-unit5-onehit-lisp-learning.ref
//! lisp: actr7.x/test-models/tutorial-unit5-onehit-lisp-learning.lisp

use crate::Case;

#[test]
// Full reason: onehit.lisp and the model are available; this needs the shared
// game driver plus the 50x20x5-hand blocked learning loop and averaging.
#[ignore = "pending: one-hit blackjack driver (sources available)"]
fn tutorial_unit5_onehit_lisp_learning() -> Result<(), String> {
    Case::new("tutorial-unit5-onehit-lisp-learning")
        .seed((100, 0))
        .check()
}
