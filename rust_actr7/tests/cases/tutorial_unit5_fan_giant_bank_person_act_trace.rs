//! case: tutorial-unit5-fan-giant-bank-person-act-trace
//! ref:  tutorial-unit5-fan-giant-bank-person-act-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-giant-bank-person-act-trace.lisp

use crate::Case;
use actr_engine::Step;

#[test]
#[ignore = "event-hook Lisp (pending filter parsing capabilities)"]
fn tutorial_unit5_fan_giant_bank_person_act_trace() -> Result<(), String> {
    Case::new("tutorial-unit5-fan-giant-bank-person-act-trace")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .seed((100, 0))
        .steps(vec![
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
