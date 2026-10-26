//! case: tutorial-unit8-simple-dynamic-lhs-productions
//! lisp: actr7.x/test-models/tutorial-unit8-simple-dynamic-lhs-productions.lisp
//! source: tutorial;unit8;simple-dynamic-model.lisp

#[test]
fn tutorial_unit8_simple_dynamic_lhs_productions() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit8;simple-dynamic-model.lisp")?;
    let actual = crate::columns(&lines, &["(p start", "(p retrieve-first"], 8)?;
    crate::check_echo("tutorial-unit8-simple-dynamic-lhs-productions", actual)
}
