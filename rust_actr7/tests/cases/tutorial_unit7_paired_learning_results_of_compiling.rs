//! case: tutorial-unit7-paired-learning-results-of-compiling
//! ref:  tutorial-unit7-paired-learning-results-of-compiling.ref
//! lisp: actr7.x/test-models/tutorial-unit7-paired-learning-results-of-compiling.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit7_paired_learning_results_of_compiling() -> Result<(), String> {
    Case::new("tutorial-unit7-paired-learning-results-of-compiling")
        .models(&[
            "tutorial;lisp;paired.lisp",
            "tutorial;unit4;paired-model.lisp",
            "tutorial;unit7;paired-learning-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::Command {
                name: "permute-list".into(),
                args: crate::sexps(r#"(("zinc" "9"))"#),
            },
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"T"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::ConsumeRng(1),
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunFullTime {
                secs: 5.0,
                trace: true,
                width: 22,
                seed: Some((100, 0)),
            },
            Step::RunNEvents {
                count: 2,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
