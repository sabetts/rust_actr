//! case: tutorial-unit3modeling-v4-print-all-visicon-items
//! ref:  tutorial-unit3modeling-v4-print-all-visicon-items.ref
//! lisp: actr7.x/test-models/tutorial-unit3modeling-v4-print-all-visicon-items.lisp

use crate::Case;
use actr_engine::Step;

fn model_settings() -> Vec<Step> {
    vec![
        Step::Sgp(":ACT".into(), crate::sexp("NIL")),
        Step::Sgp(":EPL".into(), crate::sexp("NIL")),
        Step::Sgp(":PCT".into(), crate::sexp("NIL")),
        Step::Sgp(":ALPHA".into(), crate::sexp("0.2")),
    ]
}

fn print_visicon() -> Step {
    Step::Command {
        name: "print-visicon".into(),
        args: Vec::new(),
    }
}

#[test]
fn tutorial_unit3modeling_v4_print_all_visicon_items() -> Result<(), String> {
    let mut steps = vec![Step::Reset];
    steps.extend(model_settings());
    steps.push(Step::ConsumeRng(27));
    steps.push(Step::RunUntilTime {
        time: 0.04,
        trace: false,
        width: 22,
    });
    steps.push(Step::Sgp(":V".into(), crate::sexp("T")));
    steps.push(print_visicon());

    steps.push(Step::Reset);
    steps.extend(model_settings());
    steps.push(Step::ConsumeRng(27));
    steps.push(Step::RunUntilTime {
        time: 2.3,
        trace: false,
        width: 22,
    });
    steps.push(Step::Sgp(":V".into(), crate::sexp("T")));
    steps.push(print_visicon());

    steps.push(Step::Reset);
    steps.extend(model_settings());
    steps.push(Step::ConsumeRng(27));
    steps.push(Step::Run {
        secs: 10.0,
        trace: false,
        width: 22,
    });
    steps.push(Step::Sgp(":V".into(), crate::sexp("T")));
    steps.push(print_visicon());

    Case::new("tutorial-unit3modeling-v4-print-all-visicon-items")
        .models(&[
            "tutorial;lisp;pm-issues.lisp",
            "tutorial;unit3;perceptual-motor-issues-model.lisp",
            "test-models;models;perceptual-motor-issues-model-v4.lisp",
        ])
        .seed((100, 0))
        .steps(steps)
        .check()
}
