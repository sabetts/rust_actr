//! case: tutorial-unit1code-semantic-sdm2
//! ref:  tutorial-unit1code-semantic-sdm2.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-semantic-sdm2.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit1code_semantic_sdm2() -> Result<(), String> {
    Case::new("tutorial-unit1code-semantic-sdm2")
        .models(&["tutorial;unit1;semantic.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput("? (sdm - attribute category)".into()),
            Step::ModelOutput("...".into()),
            Step::SdmNames(vec!["-".into(), "ATTRIBUTE".into(), "CATEGORY".into()]),
        ])
        .check()
}
