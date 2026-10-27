//! case: tutorial-unit5modeling-v9-spreading-activation-only
//! ref:  tutorial-unit5modeling-v9-spreading-activation-only.ref
//! lisp: actr7.x/test-models/tutorial-unit5modeling-v9-spreading-activation-only.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5modeling_v9_spreading_activation_only() -> Result<(), String> {
    Case::new("tutorial-unit5modeling-v9-spreading-activation-only")
        .models(&["test-models;models;declarative-issues-v9.lisp"])
        .seed((100, 1))
        .steps(vec![
            Step::Sgp(":V".into(), crate::sexp("NIL")),
            Step::Sgp(":SACT".into(), crate::sexp("T")),
            Step::Run {
                secs: 27.15,
                trace: false,
                width: 22,
            },
            Step::RunUntilTime {
                time: 27.381,
                trace: false,
                width: 22,
            },
            Step::SdmActivationTraces {
                spec: vec![
                    "-".into(),
                    "ARG1".into(),
                    "NIL".into(),
                    "-".into(),
                    "ARG2".into(),
                    "NIL".into(),
                ],
                time_ms: 27155,
            },
        ])
        .check()
}
