//! case: production-compilation-dynamism
//! ref:  production-compilation-dynamism.ref
//! lisp: actr7.x/test-models/production-compilation-dynamism.lisp
//! spec: fixtures/regression/specs/production-compilation-dynamism.spec

use crate::Case;

#[test]
// BLOCKED ON: cross-production mapping of dynamic slot names. The task
// driver and command spec are complete; the first divergence is the composed
// A1/B1 condition, where the p2 dynamic slot is specialized to COLOR instead
// of renamed/mapped to =S0.
#[ignore = "engine-divergence: dynamic slot mapping in composition"]
fn production_compilation_dynamism() -> Result<(), String> {
    Case::from_spec("production-compilation-dynamism").check()
}
