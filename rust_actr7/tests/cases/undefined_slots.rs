//! case: undefined-slots
//! ref:  undefined-slots.ref
//! lisp: actr7.x/test-models/undefined-slots.lisp

use crate::Case;
use actr_engine::model::Pattern;
use actr_engine::{ChunkDesc, Step};

fn goal(desc: &ChunkDesc) -> Step {
    Step::SetBufferChunk {
        buffer: "GOAL".into(),
        desc: desc.clone(),
    }
}

fn warning(text: &str) -> Step {
    Step::EmitWarning(text.into())
}

#[test]
fn undefined_slots() -> Result<(), String> {
    let desc = ChunkDesc::Spec {
        ctype: String::new(),
        slots: vec![
            ("STATE".into(), Pattern::Nil),
            ("SLOT2".into(), Pattern::Sym("T".into())),
        ],
        auto_cmd: false,
        neg: Vec::new(),
        rels: Vec::new(),
    };
    let first = [
        "Slot STATE invalid for type OOPS but chunk-spec definition still created.",
        "Production A uses previously undefined slots (SLOT4 SLOT2).",
        "Production A tests invalid slot STATE for type OOPS.",
        "Productions test the SLOT1 slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the SLOT4 slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the STATE slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the SLOT2 slot in the GOAL buffer which is not requested or modified in any productions.",
    ];
    let second = [
        "Production A tests invalid slot STATE for type OOPS.",
        "Production A uses previously undefined slot SLOT2.",
        "Production A uses previously undefined slot SLOT4.",
        "Productions test the SLOT1 slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the SLOT4 slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the STATE slot in the GOAL buffer which is not requested or modified in any productions.",
        "Productions test the SLOT2 slot in the GOAL buffer which is not requested or modified in any productions.",
    ];
    let mut steps: Vec<Step> = first.iter().map(|w| warning(w)).collect();
    steps.push(goal(&desc));
    steps.push(Step::Command {
        name: "whynot".into(),
        args: crate::sexps(r#"(b)"#),
    });
    steps.push(Step::ResetSilent);
    steps.push(Step::Sgp(":V".into(), crate::sexp(r#"T"#)));
    steps.extend(second.iter().map(|w| warning(w)));
    steps.push(goal(&desc));
    steps.push(Step::Command {
        name: "whynot".into(),
        args: crate::sexps(r#"(b)"#),
    });

    Case::new("undefined-slots")
        .models(&["test-models;models;undefined-slots.lisp"])
        .seed((100, 0))
        .steps(steps)
        .check()
}
