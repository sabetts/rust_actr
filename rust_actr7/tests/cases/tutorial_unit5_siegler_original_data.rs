//! case: tutorial-unit5-siegler-original-data
//! ref:  tutorial-unit5-siegler-original-data.ref
//! lisp: actr7.x/test-models/tutorial-unit5-siegler-original-data.lisp

use crate::check_output;

#[test]
fn tutorial_unit5_siegler_original_data() -> Result<(), String> {
    check_output("tutorial-unit5-siegler-original-data", &display_results())
}

fn display_results() -> Vec<String> {
    const DATA: [[f64; 10]; 6] = [
        [0.0, 0.05, 0.86, 0.0, 0.02, 0.0, 0.02, 0.0, 0.0, 0.06],
        [0.0, 0.04, 0.07, 0.75, 0.04, 0.0, 0.02, 0.0, 0.0, 0.09],
        [0.0, 0.02, 0.0, 0.10, 0.75, 0.05, 0.01, 0.03, 0.0, 0.06],
        [0.02, 0.0, 0.04, 0.05, 0.80, 0.04, 0.0, 0.05, 0.0, 0.0],
        [0.0, 0.0, 0.07, 0.09, 0.25, 0.45, 0.08, 0.01, 0.01, 0.06],
        [0.04, 0.0, 0.0, 0.05, 0.21, 0.09, 0.48, 0.0, 0.02, 0.11],
    ];
    const QUESTIONS: [&str; 6] = ["1+1", "1+2", "1+3", "2+2", "2+3", "3+3"];

    let mut out =
        vec!["       0     1     2     3     4     5     6     7     8   Other".to_string()];
    for (question, row) in QUESTIONS.iter().zip(DATA.iter()) {
        let mut line = question.to_string();
        for value in row {
            line.push_str(&format!("{value:6.2}"));
        }
        out.push(line);
    }
    out
}
