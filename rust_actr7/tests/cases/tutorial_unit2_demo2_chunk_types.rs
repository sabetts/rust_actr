//! case: tutorial-unit2-demo2-chunk-types
//! lisp: actr7.x/test-models/tutorial-unit2-demo2-chunk-types.lisp
//! source: tutorial;unit2;demo2-model.lisp

#[test]
fn tutorial_unit2_demo2_chunk_types() -> Result<(), String> {
    let actual = crate::echo_forms("tutorial;unit2;demo2-model.lisp", "CHUNK-TYPE", None)?;
    crate::check_echo("tutorial-unit2-demo2-chunk-types", actual)
}
