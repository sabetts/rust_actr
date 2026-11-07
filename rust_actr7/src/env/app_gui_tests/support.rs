//! Shared harnesses and scripted data for the Environment GUI tests.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::OsString;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use egui_kittest::Harness;

use crate::engine::{DeviceSetup, DeviceText};
use crate::json::Json;

use super::super::super::backend::{BackendEvent, ConnectionInfo, EnvBackend};
use super::super::super::settings::Settings;
use super::super::super::InProc;
use super::super::App;

#[derive(Clone, Debug)]
pub struct Call {
    pub name: String,
    pub args: Vec<Json>,
}

pub type CallLog = Rc<RefCell<Vec<Call>>>;

pub struct RecordingBackend {
    inner: InProc,
    pub calls: CallLog,
}

impl EnvBackend for RecordingBackend {
    fn evaluate(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.calls.borrow_mut().push(Call {
            name: name.to_string(),
            args: args.clone(),
        });
        self.inner.evaluate(name, model, args)
    }

    fn take_events(&mut self) -> Vec<BackendEvent> {
        self.inner.take_events()
    }

    fn current_model(&self) -> Option<String> {
        self.inner.current_model()
    }

    fn models(&self) -> Vec<String> {
        self.inner.models()
    }

    fn set_current_model(&mut self, name: Option<String>) {
        self.inner.set_current_model(name);
    }

    fn describe(&self) -> String {
        self.inner.describe()
    }

    fn register_window_handler(&mut self, name: &str) {
        self.inner.register_window_handler(name);
    }

    fn register_stepper_handlers(&mut self, next: &str, update: &str) -> Result<(), String> {
        self.inner.register_stepper_handlers(next, update)
    }

    fn connections(&mut self) -> Result<Vec<ConnectionInfo>, String> {
        self.inner.connections()
    }

    fn command_doc(&mut self, name: &str) -> Result<Option<String>, String> {
        self.inner.command_doc(name)
    }

    fn begin_run(&mut self, secs: f64, full: bool) -> bool {
        self.inner.begin_run(secs, full)
    }

    fn poll_run(&mut self) -> Result<bool, String> {
        self.inner.poll_run()
    }

    fn stop_run(&mut self) {
        self.inner.stop_run();
    }

    fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    fn set_real_time(&mut self, on: bool) {
        self.inner.set_real_time(on);
    }
}

#[derive(Default)]
pub struct MockState {
    pub responses: HashMap<String, VecDeque<Result<Vec<Json>, String>>>,
    pub events: VecDeque<BackendEvent>,
    pub models: Vec<String>,
    pub current_model: Option<String>,
    pub connections: Vec<ConnectionInfo>,
    pub docs: HashMap<String, Option<String>>,
    pub begin_run_result: bool,
    pub running: bool,
    pub poll_results: VecDeque<Result<bool, String>>,
    pub real_time: bool,
}

impl MockState {
    pub fn response(&mut self, name: &str, values: Vec<Json>) {
        self.responses
            .entry(name.to_string())
            .or_default()
            .push_back(Ok(values));
    }

    pub fn event(&mut self, event: BackendEvent) {
        self.events.push_back(event);
    }
}

pub struct MockBackend {
    pub calls: CallLog,
    pub state: Rc<RefCell<MockState>>,
}

impl MockBackend {
    fn record(&self, name: &str, args: &[Json]) {
        self.calls.borrow_mut().push(Call {
            name: name.to_string(),
            args: args.to_vec(),
        });
    }
}

impl EnvBackend for MockBackend {
    fn evaluate(
        &mut self,
        name: &str,
        _model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.record(name, &args);
        let mut state = self.state.borrow_mut();
        if let Some(queue) = state.responses.get_mut(name) {
            if let Some(response) = queue.pop_front() {
                return response;
            }
        }
        match name {
            "current-model" => Ok(vec![state
                .current_model
                .clone()
                .map(Json::Str)
                .unwrap_or(Json::Null)]),
            "mp-models" => Ok(vec![Json::Arr(
                state.models.iter().cloned().map(Json::Str).collect(),
            )]),
            "history-data-available" => Ok(vec![Json::Bool(true)]),
            "start-incremental-history-data" => Ok(vec![Json::Int(1)]),
            "get-incremental-history-data" => Ok(vec![Json::Str("null".into())]),
            "act-r-version" => Ok(vec![Json::Str("mock".into())]),
            _ => Ok(vec![Json::Bool(true)]),
        }
    }

    fn take_events(&mut self) -> Vec<BackendEvent> {
        self.state.borrow_mut().events.drain(..).collect()
    }

    fn current_model(&self) -> Option<String> {
        self.state.borrow().current_model.clone()
    }

    fn models(&self) -> Vec<String> {
        self.state.borrow().models.clone()
    }

    fn set_current_model(&mut self, name: Option<String>) {
        self.calls.borrow_mut().push(Call {
            name: "set-current-model".into(),
            args: vec![name.clone().map(Json::Str).unwrap_or(Json::Null)],
        });
        self.state.borrow_mut().current_model = name;
    }

    fn describe(&self) -> String {
        "mock".into()
    }

    fn register_window_handler(&mut self, _name: &str) {}

    fn register_stepper_handlers(&mut self, _next: &str, _update: &str) -> Result<(), String> {
        Ok(())
    }

    fn connections(&mut self) -> Result<Vec<ConnectionInfo>, String> {
        Ok(self.state.borrow().connections.clone())
    }

    fn command_doc(&mut self, name: &str) -> Result<Option<String>, String> {
        Ok(self.state.borrow().docs.get(name).cloned().flatten())
    }

    fn begin_run(&mut self, secs: f64, full: bool) -> bool {
        self.calls.borrow_mut().push(Call {
            name: "begin-run".into(),
            args: vec![Json::Float(secs), Json::Bool(full)],
        });
        let mut state = self.state.borrow_mut();
        state.running = state.begin_run_result;
        state.begin_run_result
    }

    fn poll_run(&mut self) -> Result<bool, String> {
        self.calls.borrow_mut().push(Call {
            name: "poll-run".into(),
            args: Vec::new(),
        });
        let mut state = self.state.borrow_mut();
        match state.poll_results.pop_front() {
            Some(result) => {
                if !matches!(result, Ok(true)) {
                    state.running = false;
                }
                result
            }
            None => {
                let running = state.running;
                if !running {
                    state.running = false;
                }
                Ok(running)
            }
        }
    }

    fn stop_run(&mut self) {
        self.calls.borrow_mut().push(Call {
            name: "stop-run".into(),
            args: Vec::new(),
        });
        self.state.borrow_mut().running = false;
    }

    fn is_running(&self) -> bool {
        self.state.borrow().running
    }

    fn set_real_time(&mut self, on: bool) {
        self.calls.borrow_mut().push(Call {
            name: "set-real-time".into(),
            args: vec![Json::Bool(on)],
        });
        self.state.borrow_mut().real_time = on;
    }
}

pub fn app_with_backend(backend: Box<dyn EnvBackend>) -> Harness<'static, App> {
    app_with_settings(
        backend,
        Settings {
            show_copyrights: false,
            ..Default::default()
        },
    )
}

pub fn app_with_settings(
    backend: Box<dyn EnvBackend>,
    settings: Settings,
) -> Harness<'static, App> {
    Harness::builder()
        .allow_missing_glyphs()
        .with_accessibility_check(false)
        .with_max_steps(64)
        .build_eframe(move |_cc| App::new(backend, None, settings))
}

pub fn inproc_harness() -> Harness<'static, App> {
    app_with_backend(Box::new(InProc::new((100, 0))))
}

pub fn device_count_harness() -> Harness<'static, App> {
    let mut backend = InProc::new((100, 0));
    backend
        .session_mut()
        .set_device(DeviceSetup {
            texts: vec![DeviceText {
                at_ms: 0,
                text: "A".into(),
                x: 100,
                y: 100,
                run: None,
                seq: 0,
            }],
            ..Default::default()
        })
        .expect("install test device");
    app_with_backend(Box::new(backend))
}

pub fn recording_harness() -> (Harness<'static, App>, CallLog) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let backend = RecordingBackend {
        inner: InProc::new((100, 0)),
        calls: calls.clone(),
    };
    (app_with_backend(Box::new(backend)), calls)
}

pub fn mock_harness() -> (Harness<'static, App>, CallLog, Rc<RefCell<MockState>>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let state = Rc::new(RefCell::new(MockState {
        models: vec!["COUNT".into()],
        current_model: Some("COUNT".into()),
        ..Default::default()
    }));
    let backend = MockBackend {
        calls: calls.clone(),
        state: state.clone(),
    };
    (app_with_backend(Box::new(backend)), calls, state)
}

pub fn model_path(relative: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("models")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

pub fn count_model() -> String {
    model_path("tutorial/unit1/count.lisp")
}

pub fn stepper_model() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("stepper")
        .join("stepper-model.lisp")
        .to_string_lossy()
        .into_owned()
}

pub fn load_model(app: &mut App, path: &str) {
    app.eval("load-act-r-model", vec![Json::Str(path.to_string())]);
}

pub fn load_count(app: &mut App) {
    load_model(app, &count_model());
}

pub fn temp_path(suffix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "actr-env-gui-{}-{}-{suffix}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

static HOME_LOCK: Mutex<()> = Mutex::new(());

/// Redirect settings/log files to a temporary home for one test.
pub struct TempHome {
    _lock: MutexGuard<'static, ()>,
    dir: PathBuf,
    old_home: Option<OsString>,
    old_userprofile: Option<OsString>,
}

impl TempHome {
    pub fn new() -> TempHome {
        let lock = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let old_home = std::env::var_os("HOME");
        let old_userprofile = std::env::var_os("USERPROFILE");
        let dir = temp_path("home");
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("HOME", &dir);
        std::env::set_var("USERPROFILE", &dir);
        TempHome {
            _lock: lock,
            dir,
            old_home,
            old_userprofile,
        }
    }
}

impl Drop for TempHome {
    fn drop(&mut self) {
        match &self.old_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match &self.old_userprofile {
            Some(value) => std::env::set_var("USERPROFILE", value),
            None => std::env::remove_var("USERPROFILE"),
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn blending_records() -> Vec<Json> {
    vec![Json::Arr(vec![
        Json::Int(300),
        Json::Str("+blending>".into()),
        Json::Str("result".into()),
        Json::Arr(vec![Json::Arr(vec![
            Json::Str("CHUNK0".into()),
            Json::Float(1.5),
            Json::Float(0.5),
            Json::Str("printed".into()),
        ])]),
        Json::Str("blend trace".into()),
    ])]
}

pub fn perceptual_records() -> Vec<Json> {
    vec![Json::Arr(vec![
        Json::Int(500),
        Json::Str("TEXT0".into()),
        Json::Str("TEXT1".into()),
    ])]
}

pub fn bold_records() -> Vec<Json> {
    vec![
        Json::Arr(vec![
            Json::Str("time".into()),
            Json::Float(0.0),
            Json::Float(100.0),
            Json::Float(200.0),
        ]),
        Json::Arr(vec![
            Json::Str("goal".into()),
            Json::Float(0.1),
            Json::Float(0.2),
            Json::Float(0.3),
        ]),
        Json::Arr(vec![
            Json::Str("retrieval".into()),
            Json::Float(0.3),
            Json::Float(0.2),
            Json::Float(0.1),
        ]),
    ]
}
