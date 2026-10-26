//! case: tutorial-unit8-bst-decide-productions
//! lisp: actr7.x/test-models/tutorial-unit8-bst-decide-productions.lisp
//! source: tutorial;unit8;bst-ppm-model.lisp

#[test]
fn tutorial_unit8_bst_decide_productions() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit8;bst-ppm-model.lisp")?;
    let actual = crate::columns(&lines, &["(p decide-over", "(p decide-under"], 8)?;
    crate::check_echo("tutorial-unit8-bst-decide-productions", actual)
}
