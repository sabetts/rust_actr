//! case: tutorial-unit3modeling-v4-add-text-code
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v4-add-text-code.lisp
//! source: tutorial;lisp;pm-issues.lisp
//! source: tutorial;python;pm_issues.py

#[test]
fn tutorial_unit3modeling_v4_add_text_code() -> Result<(), String> {
    let lisp = crate::source_lines("tutorial;lisp;pm-issues.lisp")?;
    let python = crate::source_lines("tutorial;python;pm_issues.py")?;
    let mut actual = vec!["Lisp:".to_string(), String::new()];
    actual.extend(
        lisp.iter()
            .filter(|line| line.to_lowercase().contains("(add-text-to-exp-window"))
            .cloned(),
    );
    actual.push(String::new());
    actual.push("Python:".to_string());
    actual.push(String::new());
    actual.extend(
        python
            .iter()
            .filter(|line| line.to_lowercase().contains("add_text_to_exp_window"))
            .cloned(),
    );
    crate::check_echo("tutorial-unit3modeling-v4-add-text-code", actual)
}
