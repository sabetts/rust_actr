//! case: base-level-safety-check
//! ref:  base-level-safety-check.ref
//! lisp: actr7.x/test-models/base-level-safety-check.lisp

use crate::Case;
use actr_engine::{ChunkDesc, Step};

#[test]
fn base_level_safety_check() -> Result<(), String> {
    let mut steps = Vec::new();
    for ol in ["NIL", "T", "1", "3"] {
        steps.push(Step::Reset);
        steps.push(Step::Sgp(":OL".into(), crate::sexp(ol)));
        steps.push(Step::ModelOutput("--------------------------".into()));
        steps.push(Step::AddDm(vec![ChunkDesc::Named("A".into())]));
        steps.push(Step::EmptyModuleRequest("RETRIEVAL".into()));
        push_run(&mut steps, 50.0);
        steps.push(Step::MergeDm(vec![ChunkDesc::Named("B".into())]));
        steps.push(Step::EmptyModuleRequest("RETRIEVAL".into()));
        push_run(&mut steps, 150.0);
        steps.push(Step::MergeDm(vec![ChunkDesc::Named("C".into())]));
        push_run(&mut steps, 30.0);
        steps.push(Step::EmptyModuleRequest("RETRIEVAL".into()));
        push_run(&mut steps, 10.0);
        steps.push(Step::MergeDm(vec![ChunkDesc::Named("D".into())]));
        push_run(&mut steps, 5.0);
        steps.push(Step::EmptyModuleRequest("RETRIEVAL".into()));
        push_run(&mut steps, 1.0);
        steps.push(Step::MergeDm(vec![ChunkDesc::Named("E".into())]));
        push_run(&mut steps, 50.0);
        steps.push(Step::EmptyModuleRequest("RETRIEVAL".into()));
        push_run(&mut steps, 100.0);
    }

    Case::new("base-level-safety-check")
        .models(&["test-models;models;base-level-safety-check.lisp"])
        .seed((100, 0))
        .steps(steps)
        .check()
}

fn push_run(steps: &mut Vec<Step>, secs: f64) {
    steps.push(Step::RunFullTime {
        secs,
        trace: true,
        width: 22,
        seed: Some((100, 0)),
    });
}
