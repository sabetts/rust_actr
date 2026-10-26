//! case: tutorial-unit5code-siegler-experiment-fact-base-levels
//! ref:  tutorial-unit5code-siegler-experiment-fact-base-levels.ref
//! lisp: actr7.x/test-models/tutorial-unit5code-siegler-experiment-fact-base-levels.lisp
//! spec: fixtures/regression/specs/tutorial-unit5code-siegler-experiment-fact-base-levels.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn tutorial_unit5code_siegler_experiment_fact_base_levels() -> Result<(), String> {
    Case::from_spec("tutorial-unit5code-siegler-experiment-fact-base-levels").check()
}
