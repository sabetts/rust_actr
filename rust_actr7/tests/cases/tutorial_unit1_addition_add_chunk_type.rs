//! case: tutorial-unit1-addition-add-chunk-type
//! lisp: actr7.x/test-models/tutorial-unit1-addition-add-chunk-type.lisp
//! source: tutorial;unit1;addition.lisp

#[test]
fn tutorial_unit1_addition_add_chunk_type() -> Result<(), String> {
    let actual = crate::echo_forms("tutorial;unit1;addition.lisp", "CHUNK-TYPE", Some("ADD"))?;
    crate::check_echo("tutorial-unit1-addition-add-chunk-type", actual)
}
