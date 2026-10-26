//! case: tutorial-unit5-fan-lawyer-store-trace
//! ref:  tutorial-unit5-fan-lawyer-store-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-lawyer-store-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
fn tutorial_unit5_fan_lawyer_store_trace() -> Result<(), String> {
    Case::new("tutorial-unit5-fan-lawyer-store-trace")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .seed((1, 0))
        .steps(vec![
            Step::ModelOutput("? (fan-sentence \"lawyer\" \"store\" t 'person)".into()),
            Step::ModelOutput(">>> fan.sentence('lawyer','store',True,'person')".into()),
            Step::ModelOutput("".into()),
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::Sgp(":ACT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":EPL".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":PCT".into(), crate::sexp(r#"NIL"#)),
            Step::Sgp(":ALPHA".into(), crate::sexp(r#"0.2"#)),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check()
}
