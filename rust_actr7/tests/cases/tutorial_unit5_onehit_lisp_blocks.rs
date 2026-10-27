//! case: tutorial-unit5-onehit-lisp-blocks
//! ref:  tutorial-unit5-onehit-lisp-blocks.ref
//! lisp: actr7.x/test-models/tutorial-unit5-onehit-lisp-blocks.lisp

use crate::Case;

#[test]
// Full reason: onehit.lisp and the 1-hit blackjack model are available; this
// needs a Rust port of the deck/opponent/game loop, output-key capture, and
// score/outcome helpers.
#[ignore = "pending: one-hit blackjack driver (sources available)"]
fn tutorial_unit5_onehit_lisp_blocks() -> Result<(), String> {
    Case::new("tutorial-unit5-onehit-lisp-blocks")
        .seed((100, 0))
        .check()
}
