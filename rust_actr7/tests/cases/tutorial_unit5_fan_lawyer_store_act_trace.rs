//! case: tutorial-unit5-fan-lawyer-store-act-trace
//! ref:  tutorial-unit5-fan-lawyer-store-act-trace.ref
//! lisp: actr7.x/test-models/tutorial-unit5-fan-lawyer-store-act-trace.lisp

use crate::hooks;
use actr_engine::Step;

#[test]
fn tutorial_unit5_fan_lawyer_store_act_trace() -> Result<(), String> {
    hooks::HookCase::new("tutorial-unit5-fan-lawyer-store-act-trace")
        .models(&["tutorial;lisp;fan.lisp", "tutorial;unit5;fan-model.lisp"])
        .seed((100, 0))
        .steps(vec![
            Step::Reset,
            Step::PDisable(vec!["RETRIEVE-FROM-LOCATION".into()]),
            Step::Sgp(":ACT".into(), crate::sexp("HIGH")),
            Step::Sgp(":EPL".into(), crate::sexp("NIL")),
            Step::Sgp(":PCT".into(), crate::sexp("NIL")),
            Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
            Step::Run {
                secs: 30.0,
                trace: true,
                width: 22,
            },
        ])
        .check_segment(hooks::ActTraceWindow::default())
}
