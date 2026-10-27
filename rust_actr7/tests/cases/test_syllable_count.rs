//! case: test-syllable-count
//! ref:  test-syllable-count.ref
//! lisp: actr7.x/test-models/test-syllable-count.lisp

use crate::Case;

#[test]
// Full reason: the syllable-count extra and dictionary are available, but the
// speech extension's table, split-string option, articulation-time lookup, and
// register/get/remove commands are not ported.
#[ignore = "pending: speech-extension support + driver"]
fn test_syllable_count() -> Result<(), String> {
    Case::new("test-syllable-count").seed((100, 0)).check()
}
