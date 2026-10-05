//! Error-path tests: out-of-scope Lisp must fail loudly, never silently.
//!
//! Models live in `golden/models/err-*.lisp` (project-owned fixtures,
//! loaded by path — they are not `ACT-R:` goldens and never run green).

use std::path::PathBuf;

use actr_engine::engine::{Engine, EngineError};
use actr_engine::load_models;

fn models_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden/models")
}

fn load_one(name: &str) -> actr_engine::Model {
    let mut ms = load_models(&models_dir().join(name)).unwrap();
    assert_eq!(ms.len(), 1);
    ms.pop().unwrap()
}

#[test]
fn eval_needs_lisp_and_says_so() {
    // STOP fires at the end carrying `!eval!`: the run must fail with
    // NotImplemented naming the action, after executing everything else.
    let m = load_one("err-eval.lisp");
    let engine = Engine::new(m, (100, 0), 10.0, actr_engine::DeviceSetup::default()).unwrap();
    let err = match engine.run() {
        Err(e) => e,
        Ok(_) => panic!("!eval! must not run silent"),
    };
    match err {
        EngineError::NotImplemented(msg) => assert!(
            msg.contains("!EVAL!"),
            "error should name the action: {msg}"
        ),
        other => panic!("wrong error kind: {other}"),
    }
}

#[test]
fn pas_rejected_up_front() {
    // `:pas` (permanent activation sources) is still unimplemented; the
    // config gate fires at build. (`:mas` spreading is implemented now.)
    let m = load_one("err-mas.lisp");
    let err = match Engine::new(m, (100, 0), 10.0, actr_engine::DeviceSetup::default()) {
        Err(e) => e,
        Ok(_) => panic!(":pas must be refused"),
    };
    match err {
        EngineError::NotImplemented(msg) => assert!(
            msg.contains(":PAS"),
            "error should name the parameter: {msg}"
        ),
        other => panic!("wrong error kind: {other}"),
    }
}

#[test]
fn minus_in_request_is_captured() {
    // `-` request modifiers (exclusions) are supported now: the loader
    // must capture them rather than refuse.
    use actr_engine::model::{ChunkDesc, RhsAction};
    let ms = load_models(&models_dir().join("err-reqminus.lisp")).expect("must load");
    let found = ms[0].productions.iter().any(|p| {
        p.rhs.iter().any(|a| {
            matches!(a, RhsAction::Request {
                spec: ChunkDesc::Spec { neg, .. }, ..
            } if !neg.is_empty())
        })
    });
    assert!(found, "negative request slot should be captured");
}
