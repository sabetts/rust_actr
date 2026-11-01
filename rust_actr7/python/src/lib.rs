use actr_engine::engine::Engine as CoreEngine;
use actr_engine::model::load_models_from_str;
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

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Engine>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
