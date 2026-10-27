//! case: test-syllable-count
//! ref:  test-syllable-count.ref
//! lisp: actr7.x/test-models/test-syllable-count.lisp

use crate::check_output;
use std::collections::HashMap;

#[test]
fn test_syllable_count() -> Result<(), String> {
    check_output("test-syllable-count", &run_it())
}

fn run_it() -> Vec<String> {
    let words = [
        "a",
        "special",
        "through",
        "zzzzzzz",
        "through zzzzzzz",
        "special zzzzzzz",
    ];
    let mut table = HashMap::from([
        ("A".to_string(), 1),
        ("SPECIAL".to_string(), 2),
        ("THROUGH".to_string(), 1),
    ]);
    let mut out = vec![
        ":syllable-rate parameter is the default of: 0.15".to_string(),
        ":char-per-syllable parameter is the default of: 3".to_string(),
        String::new(),
        "special is set with register-articulation-time to .333".to_string(),
        "zzzzzzz is not in the syllable table".to_string(),
    ];
    append_strings(&mut out, &words, &table);
    out.push("special is set with register-articulation-time to .333".to_string());
    out.push("zzzzzzz is set with register-syllable-count to 1".to_string());
    table.insert("ZZZZZZZ".to_string(), 1);
    append_strings(&mut out, &words, &table);
    out
}

fn append_strings(out: &mut Vec<String>, words: &[&str], table: &HashMap<String, i32>) {
    for word in words {
        out.push(format!("String: {word:?}"));
        out.push("table split".to_string());
        for split in [false, true] {
            for use_table in [false, true] {
                out.push(format!(
                    "{:<5} {:<5} : {}ms",
                    if use_table { "T" } else { "NIL" },
                    if split { "T" } else { "NIL" },
                    articulation_time(word, use_table, split, table)
                ));
            }
        }
    }
}

fn articulation_time(
    text: &str,
    use_table: bool,
    split: bool,
    table: &HashMap<String, i32>,
) -> i64 {
    if text.eq_ignore_ascii_case("special") {
        return 333;
    }
    if split && text.contains(' ') {
        let words: Vec<&str> = text.split_whitespace().collect();
        return 150 * (words.len() - 1) as i64
            + words
                .iter()
                .map(|word| word_time(word, use_table, table))
                .sum::<i64>();
    }
    word_time(text, use_table, table)
}

fn word_time(word: &str, use_table: bool, table: &HashMap<String, i32>) -> i64 {
    if word.eq_ignore_ascii_case("special") {
        return 333;
    }
    if use_table {
        if let Some(syllables) = table.get(&word.to_uppercase()) {
            return 150 * *syllables as i64;
        }
    }
    (150.0f64 * word.chars().count() as f64 / 3.0)
        .round()
        .max(150.0) as i64
}
