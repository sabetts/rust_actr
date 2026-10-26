//! case: tutorial-unit3-sperling-sgp-seed
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-sgp-seed.lisp
//! source: tutorial;unit3;sperling-model.lisp

#[test]
fn tutorial_unit3_sperling_sgp_seed() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit3;sperling-model.lisp")?;
    let line = lines
        .iter()
        .find(|line| line.to_lowercase().contains(":seed"))
        .ok_or("no :seed line")?;
    crate::check_echo(
        "tutorial-unit3-sperling-sgp-seed",
        vec![format!("; {line}")],
    )
}
