//! Python module bridge: a Python subclass of `ModuleBase` implements the
//! Rust `Module` trait through [`PyModuleAdapter`]. Python callbacks record
//! effects in [`ModuleContext`]; the adapter applies them to the real engine
//! context after the callback returns, so Python never re-enters the engine.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use actr_engine::engine::module::{
    BufferSpec, ChunkTypeSpec, Module, ModuleCtx, ModuleDescriptor, ModuleError, ModuleEvent,
    ModuleFactory, ParamSpec, ResetPhase, Value,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};

fn module_error(e: impl std::fmt::Display) -> ModuleError {
    ModuleError(e.to_string())
}

fn pyerr(e: PyErr) -> ModuleError {
    module_error(e)
}

// ------------------------------------------------------------------ values

pub(crate) fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    Ok(match value {
        Value::Int(i) => i.into_pyobject(py)?.into_any().unbind(),
        Value::Num(x) => x.into_pyobject(py)?.into_any().unbind(),
        Value::Sym(s) | Value::Str(s) => s.into_pyobject(py)?.into_any().unbind(),
        Value::Nil => py.None(),
        Value::List(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(value_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
    })
}

pub(crate) fn py_to_value(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Nil);
    }
    if let Ok(b) = value.extract::<bool>() {
        return Ok(Value::Sym(if b { "T" } else { "NIL" }.to_string()));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(x) = value.extract::<f64>() {
        return Ok(Value::Num(x));
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(Value::Sym(s));
    }
    if let Ok(iter) = value.try_iter() {
        let mut items = Vec::new();
        for item in iter {
            items.push(py_to_value(&item?)?);
        }
        return Ok(Value::List(items));
    }
    Ok(Value::Sym(value.str()?.to_string_lossy().to_string()))
}

fn slots_to_py(py: Python<'_>, slots: &[(String, Value)]) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (slot, value) in slots {
        dict.set_item(slot, value_to_py(py, value)?)?;
    }
    Ok(dict.into_any().unbind())
}

fn slots_from_py(slots: &Bound<'_, PyAny>) -> PyResult<Vec<(String, Value)>> {
    let dict = slots
        .cast::<PyDict>()
        .map_err(|_| PyTypeError::new_err("slots must be a dict"))?;
    let mut out = Vec::new();
    for (key, value) in dict.iter() {
        out.push((
            key.extract::<String>()?.to_uppercase(),
            py_to_value(&value)?,
        ));
    }
    Ok(out)
}

// ------------------------------------------------------------------ specs

#[pyclass(name = "BufferSpec", module = "rust_actr._native")]
#[derive(Clone)]
pub struct PyBufferSpec {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub spread: Option<(String, Option<f64>)>,
    #[pyo3(get, set)]
    pub request_params: Vec<String>,
    #[pyo3(get, set)]
    pub queries: Vec<String>,
    #[pyo3(get, set)]
    pub status_fn: Option<String>,
    #[pyo3(get, set)]
    pub multi: Option<String>,
}

#[pymethods]
impl PyBufferSpec {
    #[new]
    #[pyo3(signature = (name, spread=None, request_params=None, queries=None, status_fn=None, multi=None))]
    fn new(
        name: String,
        spread: Option<(String, Option<f64>)>,
        request_params: Option<Vec<String>>,
        queries: Option<Vec<String>>,
        status_fn: Option<String>,
        multi: Option<String>,
    ) -> Self {
        PyBufferSpec {
            name: name.to_uppercase(),
            spread,
            request_params: request_params.unwrap_or_default(),
            queries: queries.unwrap_or_default(),
            status_fn,
            multi,
        }
    }
}

impl PyBufferSpec {
    fn to_core(&self) -> BufferSpec {
        let multi = self
            .multi
            .as_deref()
            .and_then(|m| match m.to_ascii_lowercase().as_str() {
                "multi" => Some(actr_engine::engine::module::MultiBufferMode::Multi),
                "search" => Some(actr_engine::engine::module::MultiBufferMode::Search),
                "multi-copy" => Some(actr_engine::engine::module::MultiBufferMode::MultiCopy),
                "search-copy" => Some(actr_engine::engine::module::MultiBufferMode::SearchCopy),
                _ => None,
            });
        BufferSpec {
            name: self.name.to_uppercase(),
            spread: self.spread.clone(),
            request_params: self
                .request_params
                .iter()
                .map(|s| s.to_uppercase())
                .collect(),
            queries: self.queries.iter().map(|s| s.to_uppercase()).collect(),
            status_fn: self.status_fn.clone(),
            multi,
        }
    }
}

#[pyclass(name = "ChunkTypeSpec", module = "rust_actr._native")]
pub struct PyChunkTypeSpec {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub parents: Vec<String>,
    #[pyo3(get, set)]
    pub slots: Vec<String>,
    #[pyo3(get, set)]
    pub defaults: Option<Py<PyAny>>,
}

#[pymethods]
impl PyChunkTypeSpec {
    #[new]
    #[pyo3(signature = (name, parents=None, slots=None, defaults=None))]
    fn new(
        name: String,
        parents: Option<Vec<String>>,
        slots: Option<Vec<String>>,
        defaults: Option<Py<PyAny>>,
    ) -> Self {
        PyChunkTypeSpec {
            name: name.to_uppercase(),
            parents: parents.unwrap_or_default(),
            slots: slots.unwrap_or_default(),
            defaults,
        }
    }
}

impl PyChunkTypeSpec {
    fn to_core(&self, py: Python<'_>) -> PyResult<ChunkTypeSpec> {
        let mut defaults = Vec::new();
        if let Some(obj) = &self.defaults {
            let dict = obj
                .bind(py)
                .cast::<PyDict>()
                .map_err(|_| PyTypeError::new_err("defaults must be a dict"))?;
            for (key, value) in dict.iter() {
                defaults.push((
                    key.extract::<String>()?.to_uppercase(),
                    py_to_value(&value)?,
                ));
            }
        }
        Ok(ChunkTypeSpec {
            name: self.name.to_uppercase(),
            parents: self.parents.iter().map(|s| s.to_uppercase()).collect(),
            slots: self.slots.iter().map(|s| s.to_uppercase()).collect(),
            defaults,
        })
    }
}

#[pyclass(name = "ParamSpec", module = "rust_actr._native")]
pub struct PyParamSpec {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub owner: bool,
    #[pyo3(get, set)]
    pub default_value: Option<Py<PyAny>>,
    #[pyo3(get, set)]
    pub valid_test: Option<String>,
    #[pyo3(get, set)]
    pub warning: String,
    #[pyo3(get, set)]
    pub documentation: String,
}

#[pymethods]
impl PyParamSpec {
    #[new]
    #[pyo3(signature = (name, owner=true, default_value=None, valid_test=None, warning="".to_string(), documentation="".to_string()))]
    fn new(
        name: String,
        owner: bool,
        default_value: Option<Py<PyAny>>,
        valid_test: Option<String>,
        warning: String,
        documentation: String,
    ) -> Self {
        PyParamSpec {
            name: name.to_uppercase(),
            owner,
            default_value,
            valid_test,
            warning,
            documentation,
        }
    }
}

impl PyParamSpec {
    fn to_core(&self, py: Python<'_>) -> PyResult<ParamSpec> {
        let default = match &self.default_value {
            Some(value) => py_to_value(value.bind(py))?,
            None => Value::Nil,
        };
        Ok(ParamSpec {
            name: self.name.to_uppercase(),
            owner: self.owner,
            default,
            valid_test: self.valid_test.clone(),
            warning: self.warning.clone(),
            documentation: self.documentation.clone(),
        })
    }
}

#[pyclass(name = "ResetPhase", module = "rust_actr._native", eq, eq_int)]
#[derive(Clone, Copy, PartialEq)]
pub enum PyResetPhase {
    Primary,
    Secondary,
    Tertiary,
}

impl From<PyResetPhase> for ResetPhase {
    fn from(value: PyResetPhase) -> Self {
        match value {
            PyResetPhase::Primary => ResetPhase::Primary,
            PyResetPhase::Secondary => ResetPhase::Secondary,
            PyResetPhase::Tertiary => ResetPhase::Tertiary,
        }
    }
}

/// Base class for Python-defined modules. `_module.py` provides the default
/// no-op methods and the `Module` name.
#[pyclass(name = "ModuleBase", subclass, module = "rust_actr._native")]
pub struct ModuleBase {}

#[pymethods]
impl ModuleBase {
    #[new]
    fn new() -> Self {
        ModuleBase {}
    }
}

// --------------------------------------------------------------- effects

enum Effect {
    Emit(String),
    Busy(String, bool),
    Error(String, bool),
    SetBuffer(String, String, Vec<(String, Value)>),
    ClearBuffer(String),
    ModBuffer(String, Vec<(String, Value)>),
    ScheduleNamed(i64, String, Vec<Value>),
}

#[pyclass(name = "ModuleContext", unsendable, module = "rust_actr._native")]
pub struct ModuleContext {
    effects: Rc<RefCell<Vec<Effect>>>,
    time_ms: i64,
    params: HashMap<String, Value>,
}

#[pymethods]
impl ModuleContext {
    fn time(&self) -> f64 {
        self.time_ms as f64 / 1000.0
    }

    fn time_ms(&self) -> i64 {
        self.time_ms
    }

    fn emit(&self, details: String) {
        self.effects.borrow_mut().push(Effect::Emit(details));
    }

    fn set_busy(&self, buffer: &str, busy: bool) {
        self.effects
            .borrow_mut()
            .push(Effect::Busy(buffer.to_uppercase(), busy));
    }

    fn set_error(&self, buffer: &str, error: bool) {
        self.effects
            .borrow_mut()
            .push(Effect::Error(buffer.to_uppercase(), error));
    }

    fn set_buffer(&self, buffer: &str, ctype: &str, slots: &Bound<'_, PyAny>) -> PyResult<()> {
        let slots = slots_from_py(slots)?;
        self.effects.borrow_mut().push(Effect::SetBuffer(
            buffer.to_uppercase(),
            ctype.to_uppercase(),
            slots,
        ));
        Ok(())
    }

    fn clear_buffer(&self, buffer: &str) {
        self.effects
            .borrow_mut()
            .push(Effect::ClearBuffer(buffer.to_uppercase()));
    }

    fn mod_buffer(&self, buffer: &str, slots: &Bound<'_, PyAny>) -> PyResult<()> {
        let slots = slots_from_py(slots)?;
        self.effects
            .borrow_mut()
            .push(Effect::ModBuffer(buffer.to_uppercase(), slots));
        Ok(())
    }

    #[pyo3(signature = (delay_ms, name, payload=None))]
    fn schedule_named(
        &self,
        delay_ms: i64,
        name: String,
        payload: Option<Vec<Py<PyAny>>>,
    ) -> PyResult<()> {
        let payload = match payload {
            Some(items) => Python::attach(|py| {
                items
                    .iter()
                    .map(|item| py_to_value(item.bind(py)))
                    .collect::<PyResult<Vec<_>>>()
            })?,
            None => Vec::new(),
        };
        self.effects
            .borrow_mut()
            .push(Effect::ScheduleNamed(delay_ms, name, payload));
        Ok(())
    }

    fn param(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        match self.params.get(&name.to_uppercase()) {
            Some(value) => value_to_py(py, value),
            None => Ok(py.None()),
        }
    }
}

fn apply_effects(
    ctx: &mut ModuleCtx<'_>,
    effects: &Rc<RefCell<Vec<Effect>>>,
) -> Result<(), ModuleError> {
    let effects: Vec<Effect> = effects.borrow_mut().drain(..).collect();
    for effect in effects {
        match effect {
            Effect::Emit(details) => ctx.emit(details),
            Effect::Busy(buffer, busy) => ctx.set_busy(&buffer, busy),
            Effect::Error(buffer, error) => ctx.set_error(&buffer, error),
            Effect::SetBuffer(buffer, ctype, slots) => ctx.set_buffer(&buffer, &ctype, slots)?,
            Effect::ClearBuffer(buffer) => ctx.clear_buffer(&buffer),
            Effect::ModBuffer(buffer, slots) => ctx.mod_buffer(&buffer, slots)?,
            Effect::ScheduleNamed(delay, name, payload) => ctx.schedule_named(delay, name, payload),
        }
    }
    Ok(())
}

// --------------------------------------------------------------- requests

#[pyclass(name = "Request", module = "rust_actr._native")]
pub struct PyRequest {
    #[pyo3(get)]
    buffer: String,
    #[pyo3(get)]
    slots: Py<PyAny>,
    #[pyo3(get)]
    chunk: Option<String>,
    #[pyo3(get)]
    var_slots: Vec<String>,
}

#[pyclass(name = "ModuleEvent", module = "rust_actr._native")]
pub struct PyModuleEvent {
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    payload: Py<PyAny>,
}

// --------------------------------------------------------------- adapter

fn attr_string(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<String> {
    let value = obj.getattr(name)?;
    if value.is_callable() {
        value.call0()?.extract()
    } else {
        value.extract()
    }
}

fn attr_list(obj: &Bound<'_, PyAny>, name: &str) -> PyResult<Vec<String>> {
    let value = obj.getattr(name)?;
    let value = if value.is_callable() {
        value.call0()?
    } else {
        value
    };
    if let Ok(items) = value.extract::<Vec<Py<PyAny>>>() {
        let mut out = Vec::new();
        for item in items {
            out.push(item.bind(value.py()).extract::<String>()?);
        }
        return Ok(out);
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(vec![s]);
    }
    Ok(Vec::new())
}

fn descriptor_from(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<ModuleDescriptor> {
    let name: String = obj.call_method0("name")?.extract()?;
    let version = attr_string(obj, "version").unwrap_or_default();
    let documentation = attr_string(obj, "documentation").unwrap_or_default();
    let required = attr_list(obj, "required").unwrap_or_default();
    let requires = attr_list(obj, "requires").unwrap_or_default();

    let mut buffers = Vec::new();
    for item in obj.call_method0("buffers")?.try_iter()? {
        let item = item?;
        let spec = item.extract::<PyRef<PyBufferSpec>>()?;
        buffers.push(spec.to_core());
    }
    let mut chunk_types = Vec::new();
    for item in obj.call_method0("chunk_types")?.try_iter()? {
        let item = item?;
        let spec = item.extract::<PyRef<PyChunkTypeSpec>>()?;
        chunk_types.push(spec.to_core(py)?);
    }
    let mut params = Vec::new();
    for item in obj.call_method0("parameters")?.try_iter()? {
        let item = item?;
        let spec = item.extract::<PyRef<PyParamSpec>>()?;
        params.push(spec.to_core(py)?);
    }

    Ok(ModuleDescriptor {
        name: name.to_uppercase(),
        version,
        documentation,
        buffers,
        chunk_types,
        params,
        required,
        requires,
    })
}

pub struct PyModuleAdapter {
    obj: Py<PyAny>,
    descriptor: ModuleDescriptor,
}

impl PyModuleAdapter {
    fn call_with_ctx<F>(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        f: F,
    ) -> Result<Option<Py<PyAny>>, ModuleError>
    where
        F: for<'py> FnOnce(Python<'py>, &Bound<'py, PyAny>, Py<PyAny>) -> PyResult<Py<PyAny>>,
    {
        let effects = Rc::new(RefCell::new(Vec::new()));
        let time_ms = ctx.time_ms();
        let params: HashMap<String, Value> = self
            .descriptor
            .params
            .iter()
            .filter_map(|param| {
                ctx.param(&param.name)
                    .map(|value| (param.name.clone(), value))
            })
            .collect();
        let py_ctx = Python::attach(|py| {
            Py::new(
                py,
                ModuleContext {
                    effects: effects.clone(),
                    time_ms,
                    params,
                },
            )
        })
        .map_err(pyerr)?;
        let result = Python::attach(|py| f(py, self.obj.bind(py), py_ctx.into()));
        let result = result.map_err(pyerr)?;
        apply_effects(ctx, &effects)?;
        Ok(Some(result))
    }

    fn call_simple(&mut self, ctx: &mut ModuleCtx<'_>, method: &str) -> Result<(), ModuleError> {
        self.call_with_ctx(ctx, |py, obj, ctx| {
            let _ = py;
            obj.call_method1(method, (ctx,))?;
            Ok(obj.clone().unbind())
        })
        .map(|_| ())
    }
}

impl Module for PyModuleAdapter {
    fn name(&self) -> &str {
        &self.descriptor.name
    }

    fn version(&self) -> &str {
        &self.descriptor.version
    }

    fn documentation(&self) -> &str {
        &self.descriptor.documentation
    }

    fn buffers(&self) -> Vec<BufferSpec> {
        self.descriptor.buffers.clone()
    }

    fn chunk_types(&self) -> Vec<ChunkTypeSpec> {
        self.descriptor.chunk_types.clone()
    }

    fn parameters(&self) -> Vec<ParamSpec> {
        self.descriptor.params.clone()
    }

    fn required(&self) -> &[String] {
        &self.descriptor.required
    }

    fn requires(&self) -> &[String] {
        &self.descriptor.requires
    }

    fn create(&mut self, ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        self.call_simple(ctx, "create")
    }

    fn reset(&mut self, ctx: &mut ModuleCtx<'_>, phase: ResetPhase) -> Result<(), ModuleError> {
        let phase = match phase {
            ResetPhase::Primary => PyResetPhase::Primary,
            ResetPhase::Secondary => PyResetPhase::Secondary,
            ResetPhase::Tertiary => PyResetPhase::Tertiary,
        };
        self.call_with_ctx(ctx, |py, obj, ctx| {
            let phase = Py::new(py, phase)?;
            obj.call_method1("reset", (ctx, phase))?;
            Ok(obj.clone().unbind())
        })
        .map(|_| ())
    }

    fn request(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        request: actr_engine::engine::module::Request<'_>,
    ) -> Result<(), ModuleError> {
        let slots = Python::attach(|py| slots_to_py(py, request.slots)).map_err(pyerr)?;
        self.call_with_ctx(ctx, |py, obj, ctx| {
            let req = Py::new(
                py,
                PyRequest {
                    buffer: request.buffer.to_uppercase(),
                    slots: slots.clone_ref(py),
                    chunk: request.chunk.map(str::to_string),
                    var_slots: request.var_slots.to_vec(),
                },
            )?;
            obj.call_method1("request", (ctx, request.buffer.to_uppercase(), req))?;
            Ok(obj.clone().unbind())
        })
        .map(|_| ())
    }

    fn buffer_mod(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        request: actr_engine::engine::module::ModifyRequest<'_>,
    ) -> Result<(), ModuleError> {
        let slots = Python::attach(|py| slots_to_py(py, request.slots)).map_err(pyerr)?;
        self.call_with_ctx(ctx, |py, obj, ctx| {
            let req = Py::new(
                py,
                PyRequest {
                    buffer: request.buffer.to_uppercase(),
                    slots: slots.clone_ref(py),
                    chunk: None,
                    var_slots: Vec::new(),
                },
            )?;
            obj.call_method1("buffer_mod", (ctx, request.buffer.to_uppercase(), req))?;
            Ok(obj.clone().unbind())
        })
        .map(|_| ())
    }

    fn query(&self, buffer: &str, name: &str, value: &Value) -> Option<bool> {
        Python::attach(|py| {
            let effects = Rc::new(RefCell::new(Vec::new()));
            let ctx = Py::new(
                py,
                ModuleContext {
                    effects,
                    time_ms: 0,
                    params: HashMap::new(),
                },
            )
            .ok()?;
            let value = value_to_py(py, value).ok()?;
            let result = self
                .obj
                .bind(py)
                .call_method1("query", (ctx, buffer.to_uppercase(), name, value))
                .ok()?;
            result.extract::<bool>().ok()
        })
    }

    fn params(&mut self, ctx: &mut ModuleCtx<'_>, name: &str, value: Option<&Value>) -> Value {
        let result = if let Some(value) = value {
            let value = Python::attach(|py| value_to_py(py, value)).ok();
            self.call_with_ctx(ctx, |py, obj, ctx| {
                let result = match &value {
                    Some(value) => obj.call_method1("params", (ctx, name, value.clone_ref(py)))?,
                    None => obj.call_method1("params", (ctx, name))?,
                };
                Ok(result.unbind())
            })
        } else {
            self.call_with_ctx(ctx, |_py, obj, ctx| {
                let result = obj.call_method1("params", (ctx, name))?;
                Ok(result.unbind())
            })
        };
        match result {
            Ok(Some(value)) => {
                Python::attach(|py| py_to_value(value.bind(py))).unwrap_or(Value::Nil)
            }
            _ => Value::Nil,
        }
    }

    fn delete(&mut self, ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        self.call_simple(ctx, "delete")
    }

    fn notify_on_clear(&mut self, ctx: &mut ModuleCtx<'_>, buffer: &str, chunk: Option<&str>) {
        let _ = self.call_with_ctx(ctx, |py, obj, ctx| {
            let chunk = match chunk {
                Some(chunk) => chunk.into_pyobject(py)?.into_any().unbind(),
                None => py.None(),
            };
            obj.call_method1("notify_on_clear", (ctx, buffer.to_uppercase(), chunk))?;
            Ok(obj.clone().unbind())
        });
    }

    fn update(&mut self, ctx: &mut ModuleCtx<'_>, old_ms: i64, new_ms: i64) {
        let _ = self.call_with_ctx(ctx, |py, obj, ctx| {
            let _ = py;
            obj.call_method1(
                "update",
                (ctx, old_ms as f64 / 1000.0, new_ms as f64 / 1000.0),
            )?;
            Ok(obj.clone().unbind())
        });
    }

    fn warning(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        buffer: &str,
        request: actr_engine::engine::module::Request<'_>,
    ) {
        let slots = Python::attach(|py| slots_to_py(py, request.slots)).ok();
        let _ = self.call_with_ctx(ctx, |py, obj, ctx| {
            let req = Py::new(
                py,
                PyRequest {
                    buffer: request.buffer.to_uppercase(),
                    slots: slots
                        .as_ref()
                        .map(|s| s.clone_ref(py))
                        .unwrap_or_else(|| py.None()),
                    chunk: request.chunk.map(str::to_string),
                    var_slots: request.var_slots.to_vec(),
                },
            )?;
            obj.call_method1("warning", (ctx, buffer.to_uppercase(), req))?;
            Ok(obj.clone().unbind())
        });
    }

    fn search(&mut self, ctx: &mut ModuleCtx<'_>, buffer: &str) -> Option<Vec<String>> {
        let result = self.call_with_ctx(ctx, |_py, obj, ctx| {
            let result = obj.call_method1("search", (ctx, buffer.to_uppercase()))?;
            Ok(result.unbind())
        });
        match result {
            Ok(Some(value)) => Python::attach(|py| value.bind(py).extract::<Vec<String>>().ok()),
            _ => None,
        }
    }

    fn offset(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        buffer: &str,
        chunks: &[String],
    ) -> Option<Vec<f64>> {
        let result = self.call_with_ctx(ctx, |_py, obj, ctx| {
            let chunks = chunks.to_vec();
            let result = obj.call_method1("offset", (ctx, buffer.to_uppercase(), chunks))?;
            Ok(result.unbind())
        });
        match result {
            Ok(Some(value)) => Python::attach(|py| value.bind(py).extract::<Vec<f64>>().ok()),
            _ => None,
        }
    }

    fn run_start(&mut self, ctx: &mut ModuleCtx<'_>) {
        let _ = self.call_simple(ctx, "run_start");
    }

    fn run_end(&mut self, ctx: &mut ModuleCtx<'_>) {
        let _ = self.call_simple(ctx, "run_end");
    }

    fn on_event(&mut self, ctx: &mut ModuleCtx<'_>, event: ModuleEvent) {
        let _ = self.call_with_ctx(ctx, |py, obj, ctx| {
            let payload = PyList::empty(py);
            for value in &event.payload {
                payload.append(value_to_py(py, value)?)?;
            }
            let evt = Py::new(
                py,
                PyModuleEvent {
                    name: event.name.clone(),
                    payload: payload.into_any().unbind(),
                },
            )?;
            obj.call_method1("on_event", (ctx, evt))?;
            Ok(obj.clone().unbind())
        });
    }
}

pub struct PyModuleFactory {
    obj: Py<PyAny>,
}

impl PyModuleFactory {
    pub fn new(obj: Py<PyAny>) -> Self {
        PyModuleFactory { obj }
    }

    fn make_instance(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let obj = self.obj.bind(py);
        if obj.is_callable() {
            Ok(obj.call0()?.unbind())
        } else {
            Err(PyTypeError::new_err(
                "modules must be classes or zero-argument factories",
            ))
        }
    }
}

impl ModuleFactory for PyModuleFactory {
    fn describe(&self) -> ModuleDescriptor {
        Python::attach(|py| {
            let instance = self.make_instance(py).expect("module factory failed");
            descriptor_from(py, instance.bind(py)).expect("invalid module definition")
        })
    }

    fn create(&self) -> Box<dyn Module> {
        Python::attach(|py| {
            let obj = self
                .make_instance(py)
                .expect("module factory failed to create an instance");
            let descriptor = descriptor_from(py, obj.bind(py)).expect("invalid module definition");
            Box::new(PyModuleAdapter { obj, descriptor })
        })
    }
}
