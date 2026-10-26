//! case: tutorial-unit8-simple-ppm-model-productions
//! lisp: actr7.x/test-models/tutorial-unit8-simple-ppm-model-productions.lisp
//! source: tutorial;unit8;simple-ppm-model.lisp

#[test]
fn tutorial_unit8_simple_ppm_model_productions() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit8;simple-ppm-model.lisp")?;
    let actual = crate::columns(&lines, &["(p small", "(p medium", "(p large"], 6)?;
    crate::check_echo("tutorial-unit8-simple-ppm-model-productions", actual)
}
