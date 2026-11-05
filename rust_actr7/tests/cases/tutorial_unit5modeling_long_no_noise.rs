//! case: tutorial-unit5modeling-long-no-noise
//! ref:  tutorial-unit5modeling-long-no-noise.ref
//! lisp: actr7.x/tutorial/unit5/declarative-issues.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_long_no_noise() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-long-no-noise")
        .models(&["tutorial;unit5;declarative-issues.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":ANS".into(), crate::sexp(r#"NIL"#)),
            Step::Run {
                secs: 1.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
