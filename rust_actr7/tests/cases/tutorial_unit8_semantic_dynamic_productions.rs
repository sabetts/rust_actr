//! case: tutorial-unit8-semantic-dynamic-productions
//! lisp: actr7.x/test-models/tutorial-unit8-semantic-dynamic-productions.lisp
//! source: tutorial;unit8;semantic-dynamic-model.lisp

#[test]
fn tutorial_unit8_semantic_dynamic_productions() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit8;semantic-dynamic-model.lisp")?;
    let actual = crate::columns(&lines, &["(p retrieve-attribute", "(p direct-verify"], 8)?;
    crate::check_echo("tutorial-unit8-semantic-dynamic-productions", actual)
}
