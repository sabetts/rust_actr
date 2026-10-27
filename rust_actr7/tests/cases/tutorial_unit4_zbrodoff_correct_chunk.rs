//! case: tutorial-unit4-zbrodoff-correct-chunk
//! ref:  tutorial-unit4-zbrodoff-correct-chunk.ref
//! lisp: actr7.x/test-models/tutorial-unit4-zbrodoff-correct-chunk.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: a Zbrodoff trial driver.
// The ref needs `zbrodoff-problem "a" "2" "c" t` to build the imaginal RESULT
// chunk; this case only runs the model and dumps an empty `sdm`.
#[ignore = "driver-summary: Zbrodoff trial setup"]
fn tutorial_unit4_zbrodoff_correct_chunk() -> Result<(), String> {
    Case::new("tutorial-unit4-zbrodoff-correct-chunk")
        .models(&[
            "tutorial;lisp;zbrodoff.lisp",
            "tutorial;unit4;zbrodoff-model.lisp",
        ])
        .seed((100, 0))
        .steps(vec![
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 10.0,
                trace: false,
                width: 22,
            },
            Step::Sdm(vec!["RESULT".into(), "C".into()]),
        ])
        .check()
}
