//! case: tutorial-unit5-fan-sentence-chunks
//! lisp: actr7.x/test-models/tutorial-unit5-fan-sentence-chunks.lisp
//! source: tutorial;unit5;fan-model.lisp

#[test]
fn tutorial_unit5_fan_sentence_chunks() -> Result<(), String> {
    let lines = crate::source_lines("tutorial;unit5;fan-model.lisp")?;
    let mut actual = Vec::new();
    let mut started = false;
    for line in &lines {
        let low = line.to_lowercase();
        if low.contains("(add-dm") || low.contains("isa comprehend-") {
            started = true;
            actual.push(line.clone());
        } else if started {
            actual.push("   ...)".to_string());
            break;
        }
    }
    crate::check_echo("tutorial-unit5-fan-sentence-chunks", actual)
}
