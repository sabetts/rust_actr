//! case: tutorial-unit3-sperling-sgp-v
//! lisp: actr7.x/test-models/tutorial-unit3-sperling-sgp-v.lisp
//! source: tutorial;unit3;sperling-model.lisp

#[test]
fn tutorial_unit3_sperling_sgp_v() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit3;sperling-model.lisp")?;
    let line = lines
        .iter()
        .find(|line| line.to_lowercase().contains(":v "))
        .ok_or("no :v line")?;
    let at = line.find(":v ").ok_or("no :v offset")?;
    let actual = format!("{} nil {}", &line[..at + 2], &line[at + 5..]);
    crate::check_echo("tutorial-unit3-sperling-sgp-v", vec![actual])
}
