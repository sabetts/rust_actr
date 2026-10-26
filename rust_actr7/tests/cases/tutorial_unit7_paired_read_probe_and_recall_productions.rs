//! case: tutorial-unit7-paired-read-probe-and-recall-productions
//! lisp: actr7.x/test-models/tutorial-unit7-paired-read-probe-and-recall-productions.lisp
//! source: tutorial;unit4;paired-model.lisp

#[test]
fn tutorial_unit7_paired_read_probe_and_recall_productions() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit4;paired-model.lisp")?;
    let actual = crate::columns(&lines, &["(p read-probe", "(p recall"], 5)?;
    crate::check_echo(
        "tutorial-unit7-paired-read-probe-and-recall-productions",
        actual,
    )
}
