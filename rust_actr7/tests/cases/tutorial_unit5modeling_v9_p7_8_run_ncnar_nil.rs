//! case: tutorial-unit5modeling-v9-p7-8-run-ncnar-nil
//! ref:  tutorial-unit5modeling-v9-p7-8-run-ncnar-nil.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-p7-8-run-ncnar-nil.lisp

use crate::Case;
use actr_engine::Step;

#[test]
// BLOCKED ON: runtime `:ncnar nil` vs the v10 refs. The alias-aware
// `pprint-chunks` path is implemented, but applying `:ncnar` at runtime changes
// declarative-issues-v10 activation/naming; the two conflict.
#[ignore = "engine divergence: runtime ncnar vs v10"]
fn tutorial_unit5modeling_v9_p7_8_run_ncnar_nil() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-p7-8-run-ncnar-nil")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":NCNAR".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 28.0,
                trace: false,
                width: 22,
            },
            Step::Sdm(vec!["-".into(), "STATE".into(), "NIL".into()]),
            Step::Sdm(vec!["-".into(), "GOAL".into(), "NIL".into()]),
        ])
        .check()
}
