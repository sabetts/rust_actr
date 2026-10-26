//! case: tutorial-unit7-paired-learning-encode-arg1-and-retrieve-operator-productions
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-encode-arg1-and-retrieve-operator-productions.lisp
//! source: tutorial;unit7;paired-learning-model.lisp

#[test]
fn tutorial_unit7_paired_learning_encode_arg1_and_retrieve_operator_productions(
) -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit7;paired-learning-model.lisp")?;
    let actual = crate::columns(&lines, &["(p encode-arg1", "(p retrieve-operator"], 5)?;
    crate::check_echo(
        "tutorial-unit7-paired-learning-encode-arg1-and-retrieve-operator-productions",
        actual,
    )
}
