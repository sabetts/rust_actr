//! Dispatch-aware model loading for overridden custom-module commands.
//!
//! Milestone 1 of `planning/RPC-MODULES-WORK.md` only needs the loader path:
//! model-body `add-dm`, `define-chunks`, `merge-chunks`, and `goal-focus`
//! reach a remote command during load/reset, without the built-in static path
//! applying them a second time.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use actr_engine::json::Json;
use actr_engine::rpc::session::{Remote, Session};
use actr_engine::{BufferSpec, Module, ModuleCtx, ModuleError, ModuleValue, ResetPhase};

#[derive(Debug, Clone)]
struct Call {
    name: String,
    model: Option<String>,
    args: Vec<Json>,
}

#[derive(Default)]
struct RecordingRemote {
    calls: Vec<Call>,
    fail_on: Option<String>,
    log: Option<Rc<RefCell<Vec<String>>>>,
}

impl RecordingRemote {
    fn with_log(log: Rc<RefCell<Vec<String>>>) -> Self {
        RecordingRemote {
            log: Some(log),
            ..Default::default()
        }
    }

    fn record(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        if let Some(log) = &self.log {
            log.borrow_mut().push(format!("callback:{name}"));
        }
        self.calls.push(Call {
            name: name.to_string(),
            model: model.map(str::to_string),
            args,
        });
        if self.fail_on.as_deref() == Some(name) {
            return Err(format!("forced failure for {name}"));
        }
        Ok(vec![Json::Null])
    }
}

impl Remote for RecordingRemote {
    fn call(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.record(name, model, args)
    }

    fn call_from(
        &mut self,
        _owner: Option<&str>,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.record(name, model, args)
    }
}

struct PhaseProbe {
    log: Rc<RefCell<Vec<String>>>,
}

impl Module for PhaseProbe {
    fn name(&self) -> &str {
        "phase-probe"
    }

    fn buffers(&self) -> Vec<BufferSpec> {
        vec![BufferSpec::new("retrieval")]
    }

    fn query(&self, _buffer: &str, _name: &str, _value: &ModuleValue) -> Option<bool> {
        None
    }

    fn create(&mut self, _ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        self.log.borrow_mut().push("create".to_string());
        Ok(())
    }

    fn reset(&mut self, _ctx: &mut ModuleCtx<'_>, phase: ResetPhase) -> Result<(), ModuleError> {
        self.log.borrow_mut().push(format!("{phase:?}"));
        Ok(())
    }
}

fn fixture_model(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("models")
        .join(name.replace(';', std::path::MAIN_SEPARATOR_STR))
        .to_string_lossy()
        .into_owned()
}

fn count_model() -> String {
    fixture_model("tutorial;unit1;count.lisp")
}

fn load(session: &mut Session, remote: &mut RecordingRemote, path: &str) -> Result<(), String> {
    session
        .evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(path.to_string())],
            remote,
        )
        .map(|_| ())
}

fn dm_chunks(session: &mut Session, remote: &mut RecordingRemote) -> Vec<String> {
    let result = session.evaluate("dm", None, &[], remote).unwrap();
    match result.first() {
        Some(Json::Arr(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn add_count_overrides(session: &mut Session) {
    session.add_remote("add-dm", None, Some("connection-1".to_string()));
    session.add_remote("goal-focus", None, Some("connection-1".to_string()));
}

#[test]
fn load_dispatches_overridden_add_dm_and_goal_focus() {
    let mut session = Session::new((100, 0));
    add_count_overrides(&mut session);
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();

    let names: Vec<&str> = remote.calls.iter().map(|call| call.name.as_str()).collect();
    assert_eq!(names, vec!["add-dm", "goal-focus"]);

    let add_dm = &remote.calls[0];
    assert_eq!(add_dm.model.as_deref(), Some("COUNT"));
    assert_eq!(add_dm.args.len(), 5, "count.lisp has five add-dm chunks");

    let goal = &remote.calls[1];
    assert_eq!(goal.model.as_deref(), Some("COUNT"));
    assert_eq!(goal.args.len(), 1);
    assert!(matches!(goal.args[0], Json::Arr(_)), "goal-focus spec");

    // The built-in DM path must not also consume the model-body add-dm.
    assert!(dm_chunks(&mut session, &mut remote).is_empty());
}

#[test]
fn reset_dispatches_the_same_load_actions_again() {
    let mut session = Session::new((100, 0));
    add_count_overrides(&mut session);
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();
    remote.calls.clear();

    session.evaluate("reset", None, &[], &mut remote).unwrap();

    let names: Vec<&str> = remote.calls.iter().map(|call| call.name.as_str()).collect();
    assert_eq!(names, vec!["add-dm", "goal-focus"]);
    assert!(dm_chunks(&mut session, &mut remote).is_empty());
}

#[test]
fn no_override_keeps_the_static_fast_path() {
    let mut session = Session::new((100, 0));
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();

    assert!(remote.calls.is_empty());
    assert_eq!(dm_chunks(&mut session, &mut remote).len(), 5);
}

#[test]
fn partial_override_dispatches_only_the_overridden_form() {
    let mut session = Session::new((100, 0));
    session.add_remote("goal-focus", None, Some("connection-1".to_string()));
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();

    let names: Vec<&str> = remote.calls.iter().map(|call| call.name.as_str()).collect();
    assert_eq!(names, vec!["goal-focus"]);
    assert_eq!(dm_chunks(&mut session, &mut remote).len(), 5);
}

#[test]
fn in_process_module_without_override_does_not_dispatch() {
    let mut session = Session::new((100, 0));
    let log = Rc::new(RefCell::new(Vec::new()));
    let module_log = log.clone();
    session
        .add_module_factory(Box::new(move || {
            Box::new(PhaseProbe {
                log: module_log.clone(),
            }) as Box<dyn Module>
        }))
        .unwrap();
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();

    assert!(remote.calls.is_empty());
    assert_eq!(dm_chunks(&mut session, &mut remote).len(), 5);
    assert!(log.borrow().contains(&"create".to_string()));
}

#[test]
fn load_actions_run_between_secondary_and_tertiary_reset() {
    let mut session = Session::new((100, 0));
    let log = Rc::new(RefCell::new(Vec::new()));
    let module_log = log.clone();
    session
        .add_module_factory(Box::new(move || {
            Box::new(PhaseProbe {
                log: module_log.clone(),
            }) as Box<dyn Module>
        }))
        .unwrap();
    add_count_overrides(&mut session);
    let mut remote = RecordingRemote::with_log(log.clone());

    load(&mut session, &mut remote, &count_model()).unwrap();

    let expected = [
        "create",
        "Primary",
        "Secondary",
        "callback:add-dm",
        "callback:goal-focus",
        "Tertiary",
    ]
    .iter()
    .map(|entry| entry.to_string())
    .collect::<Vec<_>>();
    assert_eq!(*log.borrow(), expected);
}

#[test]
fn dispatch_error_removes_the_partially_loaded_model() {
    let mut session = Session::new((100, 0));
    add_count_overrides(&mut session);
    let mut remote = RecordingRemote {
        fail_on: Some("add-dm".to_string()),
        ..Default::default()
    };

    let error = load(&mut session, &mut remote, &count_model()).unwrap_err();
    assert!(error.contains("forced failure"));
    assert!(session.models().is_empty());
    assert!(session.current_model().is_none());
}

#[test]
fn engine_rebuild_does_not_replay_load_actions() {
    let mut session = Session::new((100, 0));
    add_count_overrides(&mut session);
    let mut remote = RecordingRemote::default();

    load(&mut session, &mut remote, &count_model()).unwrap();
    remote.calls.clear();

    session
        .evaluate(
            "chunk-type",
            None,
            &[
                Json::Str("probe-type".to_string()),
                Json::Str("slot".to_string()),
            ],
            &mut remote,
        )
        .unwrap();

    assert!(remote.calls.is_empty());
}

#[test]
fn multi_model_load_dispatches_each_models_actions() {
    let mut session = Session::new((100, 0));
    session.add_remote("add-dm", None, Some("connection-1".to_string()));
    session.add_remote("goal-focus", None, Some("connection-1".to_string()));
    let mut remote = RecordingRemote::default();

    load(
        &mut session,
        &mut remote,
        &fixture_model("examples;multiple-models;unit-1-together-1-mp.lisp"),
    )
    .unwrap();

    let models: Vec<String> = remote
        .calls
        .iter()
        .filter(|call| call.name == "add-dm")
        .filter_map(|call| call.model.clone())
        .collect();
    assert!(
        models.len() >= 3,
        "expected one add-dm per model, got {models:?}"
    );
    assert!(models.iter().any(|name| name.eq_ignore_ascii_case("COUNT")));
    assert!(models
        .iter()
        .any(|name| name.eq_ignore_ascii_case("ADDITION")));
    assert!(models
        .iter()
        .any(|name| name.eq_ignore_ascii_case("SEMANTIC")));
}
