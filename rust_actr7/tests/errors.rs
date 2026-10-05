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
fn mas_rejected_up_front() {
    // No point running: the config gate fires at build.
    let m = load_one("err-mas.lisp");
    let err = match Engine::new(m, (100, 0), 10.0, actr_engine::DeviceSetup::default()) {
        Err(e) => e,
        Ok(_) => panic!(":mas must be refused"),
    };
    match err {
        EngineError::NotImplemented(msg) => assert!(
            msg.contains(":MAS"),
            "error should name the parameter: {msg}"
        ),
        other => panic!("wrong error kind: {other}"),
    }
}

#[test]
fn minus_in_request_rejected_at_load() {
    // `-` request modifiers are valid ACT-R (match chunks lacking the
    // slot) but unimplemented: the loader must refuse, not mis-parse.
    let err = match load_models(&models_dir().join("err-reqminus.lisp")) {
        Err(e) => e,
        Ok(_) => panic!("must not load"),
    };
    let msg = err.to_string();
    assert!(msg.contains("negation"), "error should explain: {msg}");
}
