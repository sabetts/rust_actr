//! case: tutorial-unit1code-sgp-result
//! ref:  tutorial-unit1code-sgp-result.ref
//! lisp: actr7.x/test-models/tutorial-unit1code-sgp-result.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "command-layer output (pending promotion)"]
fn tutorial_unit1code_sgp_result() -> Result<(), String> {
    Case::new("tutorial-unit1code-sgp-result")
        .models(&["tutorial;unit1;tutor-model-solution.lisp"])
        .seed((100, 0))
        .steps(vec![Step::ModelOutput("? (sgp :trace-detail :lf)".into())])
        .check()
}
