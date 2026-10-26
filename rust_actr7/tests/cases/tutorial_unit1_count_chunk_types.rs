//! case: tutorial-unit1-count-chunk-types
//! lisp: actr7.x/test-models/tutorial-unit1-count-chunk-types.lisp
//! source: tutorial;unit1;count.lisp

#[test]
fn tutorial_unit1_count_chunk_types() -> Result<(), String> {
    let actual = crate::echo_forms("tutorial;unit1;count.lisp", "CHUNK-TYPE", None)?;
    crate::check_echo("tutorial-unit1-count-chunk-types", actual)
}
