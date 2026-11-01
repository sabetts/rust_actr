use std::path::PathBuf;
use std::rc::Rc;

use actr_engine::engine::Engine as CoreEngine;
use actr_engine::json::Json;
use actr_engine::model::load_models_from_str;
use actr_engine::rpc::session::{MonitorWhen, Remote, Session as CoreSession};
use actr_engine::trace::MODULE_WIDTH;
use actr_engine::DeviceSetup;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

fn runtime_error(e: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

fn take_since(engine: &CoreEngine, start: usize) -> String {
    let lines = engine.out_since(start);
    if lines.is_empty() {
        return String::new();
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn default_root() -> PathBuf {
    std::env::var("ACTR_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("actr7.x"))
}

fn parse_args(text: &str) -> PyResult<Vec<Json>> {
    let value = actr_engine::json::parse(text).map_err(PyValueError::new_err)?;
    Ok(match value {
        Json::Arr(items) => items,
        Json::Null => Vec::new(),
        other => vec![other],
    })
}

/// A persistent ACT-R engine built from a complete `define-model` source.
#[pyclass(unsendable, module = "rust_actr._native")]
struct Engine {
    inner: CoreEngine,
}

#[pymethods]
impl Engine {
    #[new]
    #[pyo3(signature = (source, seed0=100, seed1=0))]
    fn new(source: &str, seed0: u64, seed1: u64) -> PyResult<Self> {
        let models = load_models_from_str(source).map_err(runtime_error)?;
        let model = models
            .last()
            .cloned()
            .ok_or_else(|| PyValueError::new_err("source contains no define-model"))?;
        let inner = CoreEngine::new_with_width(
            model,
            (seed0, seed1),
            0.0,
            DeviceSetup::default(),
            MODULE_WIDTH,
        )
        .map_err(runtime_error)?;
        Ok(Self { inner })
    }

    /// Run for `seconds` of model time; return only the newly emitted trace.
    fn run(&mut self, seconds: f64) -> PyResult<String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(PyValueError::new_err(
                "seconds must be a finite non-negative number",
            ));
        }
        let start = self.inner.out_len();
        self.inner.run_secs(seconds).map_err(runtime_error)?;
        Ok(take_since(&self.inner, start))
    }

    /// Run to an absolute model time; return the newly emitted trace.
    fn run_until(&mut self, time: f64) -> PyResult<String> {
        if !time.is_finite() || time < 0.0 {
            return Err(PyValueError::new_err(
                "time must be a finite non-negative number",
            ));
        }
        let start = self.inner.out_len();
        self.inner.run_until(time).map_err(runtime_error)?;
        Ok(take_since(&self.inner, start))
    }

    /// Reset the engine; return the trace emitted by the reset.
    fn reset(&mut self) -> PyResult<String> {
        let start = self.inner.out_len();
        self.inner.reset().map_err(runtime_error)?;
        Ok(take_since(&self.inner, start))
    }

    /// All trace emitted since the engine was created.
    fn trace(&self) -> String {
        self.inner.output()
    }

    /// Return the complete trace and clear the history.
    fn take_trace(&mut self) -> String {
        let text = self.inner.output();
        self.inner.set_out_lines(Vec::new());
        text
    }

    fn clear_trace(&mut self) {
        self.inner.set_out_lines(Vec::new());
    }

    fn set_trace(&mut self, enabled: bool) {
        self.inner.set_trace(enabled);
    }

    fn time(&self) -> f64 {
        self.inner.time_secs()
    }
}

/// A command-layer session: the same surface the RPC server drives, usable
/// in-process.  Runtime commands go through [`CoreSession::evaluate`], with
/// arguments and results exchanged as JSON.
struct NoRemote;

impl Remote for NoRemote {
    fn call(
        &mut self,
        _name: &str,
        _model: Option<&str>,
        _args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        // Python-owned commands are dispatched by the Python facade after the
        // engine call returns; the native session never needs to re-enter
        // Python here.
        Ok(vec![Json::Null])
    }
}

#[pyclass(unsendable, module = "rust_actr._native")]
struct Session {
    inner: CoreSession,
}

#[pymethods]
impl Session {
    #[new]
    #[pyo3(signature = (source=None, seed0=100, seed1=0, root=None))]
    fn new(
        source: Option<&str>,
        seed0: u64,
        seed1: u64,
        root: Option<&str>,
    ) -> PyResult<Self> {
        let root = root.map(PathBuf::from).unwrap_or_else(default_root);
        let mut inner = CoreSession::new(root, (seed0, seed1));
        if let Some(src) = source {
            inner.load_source(src).map_err(runtime_error)?;
        }
        Ok(Self { inner })
    }

    /// Load model definitions from in-memory Lisp source.
    fn load_source(&mut self, source: &str) -> PyResult<()> {
        self.inner.load_source(source).map_err(runtime_error)
    }

    /// Install a Python `:sim-hook`.  The callable receives the two printed
    /// values and returns a number (or `None` to fall through).  It must not
    /// call back into the model.
    fn set_sim_hook(&mut self, callable: Option<Py<PyAny>>) -> PyResult<()> {
        let engine = self
            .inner
            .engine_mut()
            .ok_or_else(|| PyRuntimeError::new_err("no model loaded"))?;
        let hook = callable.map(|callable| {
            Rc::new(move |a: &str, b: &str| {
                Python::attach(|py| {
                    let result = callable.call1(py, (a, b)).ok()?;
                    result.bind(py).extract::<f32>().ok()
                })
            }) as Rc<dyn Fn(&str, &str) -> Option<f32>>
        });
        engine.set_sim_hook(hook);
        Ok(())
    }

    /// Evaluate a command.  `args_json` is a JSON array; the result is a JSON
    /// array (the same wire representation the RPC server uses).
    #[pyo3(signature = (name, args_json="[]"))]
    fn evaluate(&mut self, name: &str, args_json: &str) -> PyResult<String> {
        let args = parse_args(args_json)?;
        let mut remote = NoRemote;
        let out = self
            .inner
            .evaluate(name, None, &args, &mut remote)
            .map_err(runtime_error)?;
        Ok(Json::Arr(out).write())
    }

    fn add_remote(&mut self, name: &str) {
        self.inner.add_remote(name, None, None);
    }

    fn remove_command(&mut self, name: &str) -> bool {
        self.inner.remove(name)
    }

    fn add_monitor(&mut self, command: &str, monitor: &str) {
        self.inner
            .add_monitor(command, monitor, MonitorWhen::Simple);
    }

    fn remove_monitor(&mut self, command: &str, monitor: &str) {
        self.inner.remove_monitor(command, monitor);
    }

    /// Commands the model fired (`output-key`, ...), as `(name, model, key)`.
    fn take_fired_commands(&mut self) -> Vec<(String, String, String)> {
        self.inner.take_fired_commands()
    }

    /// Task commands requested by `!eval!`/`!safe-eval!`, with rendered args.
    fn take_fired_task_commands(&mut self) -> Vec<(String, Vec<String>)> {
        self.inner.take_fired_task_commands()
    }

    /// New trace since the previous call.
    fn take_trace(&mut self) -> String {
        self.inner.take_trace_output()
    }

    fn time(&self) -> f64 {
        self.inner.time_secs()
    }

    /// Execute one engine event (no run terminator).
    fn step_once(&mut self) -> PyResult<bool> {
        self.inner.step_once().map_err(runtime_error)
    }

    /// Prepare an interleaved run. Returns `(limit_ms, full, should_run)`.
    fn run_begin(&mut self, name: &str, secs: f64) -> (i64, bool, bool) {
        self.inner.run_begin(name, secs)
    }

    /// One slice of an interleaved run; `true` when it yielded.
    fn run_slice(&mut self, limit_ms: i64, full: bool) -> PyResult<bool> {
        self.inner.run_slice(limit_ms, full).map_err(runtime_error)
    }

    /// One bounded chunk of a run.  With `max_events=1` this lets a Python
    /// caller dispatch task commands between engine events.  Returns `true`
    /// while the run has more work (it was event-capped); `false` once the
    /// run finished and emitted its stop terminator.
    #[pyo3(signature = (limit_ms, full, max_events=None, soft_ms=None))]
    fn run_chunk(
        &mut self,
        limit_ms: i64,
        full: bool,
        max_events: Option<u64>,
        soft_ms: Option<i64>,
    ) -> PyResult<bool> {
        self.inner
            .run_chunk(limit_ms, full, max_events, soft_ms)
            .map_err(runtime_error)
    }

    /// Events evaluated in the last run (the `run` command's second value).
    fn run_events(&self) -> u64 {
        self.inner.run_events()
    }
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Engine>()?;
    m.add_class::<Session>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
