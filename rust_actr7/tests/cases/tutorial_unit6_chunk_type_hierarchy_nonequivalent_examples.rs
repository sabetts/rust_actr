//! case: tutorial-unit6-chunk-type-hierarchy-nonequivalent-examples
//! lisp: actr7.x/test-models/tutorial-unit6-chunk-type-hierarchy-nonequivalent-examples.lisp
//! source: test-models;models;unit6-chunk-type-hierarchy.lisp

#[test]
fn tutorial_unit6_chunk_type_hierarchy_nonequivalent_examples() -> Result<(), String> {
    let lines = crate::source_lines("test-models;models;unit6-chunk-type-hierarchy.lisp")?;
    let actual = lines
        .iter()
        .filter(|line| line.to_lowercase().contains(";(chunk-type"))
        .map(|line| line.replace(';', ""))
        .collect();
    crate::check_echo(
        "tutorial-unit6-chunk-type-hierarchy-nonequivalent-examples",
        actual,
    )
}
