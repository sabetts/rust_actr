//! case: tutorial-unit5code-siegler-experiment-esc-nil
//! ref:  tutorial-unit5code-siegler-experiment-esc-nil.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-esc-nil.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5code_siegler_experiment_esc_nil() -> Result<(), String> {
    Case::new("tutorial-unit5code-siegler-experiment-esc-nil")
        .models(&["tutorial;lisp;siegler.lisp", "tutorial;unit5;siegler-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":STARTING-PARAMETERS".into(), crate::sexp(r#"(:SEED 107)"#)),
            Step::Reset,
            Step::Seed((102, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Reset,
            Step::Seed((103, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Reset,
            Step::Seed((104, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Reset,
            Step::Seed((105, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Reset,
            Step::Seed((106, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Reset,
            Step::Seed((107, 0)),
            Step::Sgp(":ESC".into(), crate::sexp(r#"NIL"#)),
            Step::Run { secs: 30.0, trace: false, width: 22 },
            Step::Command { name: "correlation".into(), args: crate::sexps(r#"(((0 0 1 0 0 0 0 0 0 0) (0 0 0 1 0 0 0 0 0 0) (0 0 0 0 1 0 0 0 0 0) (0 0 0 0 1 0 0 0 0 0) (0 0 0 0 0 1 0 0 0 0) (0 0 0 0 0 0 1 0 0 0)) ((0 0.05 0.86 0 0.02 0 0.02 0 0 0.06) (0 0.04 0.07 0.75 0.04 0 0.02 0 0 0.09) (0 0.02 0 0.1 0.75 0.05 0.01 0.03 0 0.06) (0.02 0 0.04 0.05 0.8 0.04 0 0.05 0 0) (0 0 0.07 0.09 0.25 0.45 0.08 0.01 0.01 0.06) (0.04 0 0 0.05 0.21 0.09 0.48 0 0.02 0.11)))"#) },
            Step::Command { name: "mean-deviation".into(), args: crate::sexps(r#"(((0 0 1 0 0 0 0 0 0 0) (0 0 0 1 0 0 0 0 0 0) (0 0 0 0 1 0 0 0 0 0) (0 0 0 0 1 0 0 0 0 0) (0 0 0 0 0 1 0 0 0 0) (0 0 0 0 0 0 1 0 0 0)) ((0 0.05 0.86 0 0.02 0 0.02 0 0 0.06) (0 0.04 0.07 0.75 0.04 0 0.02 0 0 0.09) (0 0.02 0 0.1 0.75 0.05 0.01 0.03 0 0.06) (0.02 0 0.04 0.05 0.8 0.04 0 0.05 0 0) (0 0 0.07 0.09 0.25 0.45 0.08 0.01 0.01 0.06) (0.04 0 0 0.05 0.21 0.09 0.48 0 0.02 0.11)))"#) },
        ])
        .check()
}
