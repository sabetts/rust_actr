//! case: production-compilation-dynamism
//! ref:  production-compilation-dynamism.ref
//! lisp: actr7.x/test-models/production-compilation-dynamism.lisp
//! spec: fixtures/regression/specs/production-compilation-dynamism.spec

use crate::Case;

#[test]
#[ignore = "pending: does not yet match ref"]
fn production_compilation_dynamism() -> Result<(), String> {
    Case::from_spec("production-compilation-dynamism").check()
}
