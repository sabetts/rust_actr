//! case: tutorial-unit1code-semantic-sdm
//! ref:  tutorial-unit1code-semantic-sdm.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-sdm.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_sdm() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-sdm")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (sdm object shark)".into()),
            Step::Sdm(vec!["OBJECT".into(), "SHARK".into()]),
            Step::ModelOutput("(P1 P2 P3)".into()),
            Step::ModelOutput("".into()),
            Step::ModelOutput("? (sdm - attribute category - attribute nil)".into()),
            Step::Sdm(vec![
                "-".into(),
                "ATTRIBUTE".into(),
                "CATEGORY".into(),
                "-".into(),
                "ATTRIBUTE".into(),
                "NIL".into(),
            ]),
            Step::ModelOutput("(P1 P2 P4 P5 P7 P8 P10 P11 P12 P13 P15 P16 P18 P19)".into()),
        ])
        .check()
}
