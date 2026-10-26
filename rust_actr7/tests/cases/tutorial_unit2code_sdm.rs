//! case: tutorial_unit2code_sdm
//! ref:  tutorial_unit2code_sdm.ref
//! lisp: actr7.x/test-models/tutorial_unit2code_sdm.py

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit2code_sdm() -> Result<(), String> {
    Case::new("tutorial_unit2code_sdm")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::ModelOutput(">>> actr.sdm('-','addend1','1')".into()),
            Step::Sdm(vec!["-".into(), "ADDEND1".into(), "1".into()]),
            Step::ModelOutput("['FACT34', 'FACT67', 'FACT103']".into()),
        ])
        .check()
}
