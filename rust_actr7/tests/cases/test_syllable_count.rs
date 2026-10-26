//! case: test-syllable-count
//! ref:  test-syllable-count.ref
//! lisp: actr7.x/test-models/test-syllable-count.lisp

use crate::Case;

#[test]
#[ignore = "arbitrary-Lisp command stub (pending translation to rust)"]
fn test_syllable_count() -> Result<(), String> {
    Case::new("test-syllable-count").seed((100, 0)).check()
}
