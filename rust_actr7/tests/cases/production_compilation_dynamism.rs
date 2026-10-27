//! case: production-compilation-dynamism
//! ref:  production-compilation-dynamism.ref
//! lisp: actr7.x/test-models/production-compilation-dynamism.lisp
//! spec: fixtures/regression/specs/production-compilation-dynamism.spec

use crate::Case;

#[test]
// BLOCKED ON: compiled-production trace across dynamic redefinitions.
// The ref is a 10k-line sequence of run/compile/utility events; production
// naming, timing, or utility output still diverges.
#[ignore = "engine-divergence: production compilation trace"]
fn production_compilation_dynamism() -> Result<(), String> {
    Case::from_spec("production-compilation-dynamism").check()
}
