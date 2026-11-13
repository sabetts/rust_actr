//! Rust-defined ACT-R modules.
//!
//! This is the Rust counterpart of Lisp's `define-module`: a module declares
//! its buffers, parameters and chunk types, and the engine routes requests,
//! queries and parameter access to it. Modules are registered through a
//! [`ModuleFactory`] so every model/engine gets its own instance.

use std::collections::HashSet;

use super::{bad, Event, Out, Prio, Value};
use crate::model::{ChunkDesc, ChunkType, Pattern};

/// The reset hook being invoked (`:reset`'s primary/secondary/tertiary
/// callbacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetPhase {
    Primary,
    Secondary,
    Tertiary,
}

/// How a multi-buffer is searched (`:multi`, `:search`, `:multi-copy`,
/// `:search-copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiBufferMode {
    Multi,
    Search,
    MultiCopy,
    SearchCopy,
}

/// One entry in a module's buffer list (`define-buffer`).
#[derive(Debug, Clone)]
pub struct BufferSpec {
    pub name: String,
    /// Spreading-activation parameter name and default (`:param-name` /
    /// `:param-default`).
    pub spread: Option<(String, Option<f64>)>,
    /// Request parameters accepted by the buffer.
    pub request_params: Vec<String>,
    /// Extra queries answered by the module through [`Module::query`].
    pub queries: Vec<String>,
    /// Optional `buffer-status` printing command.
    pub status_fn: Option<String>,
    pub multi: Option<MultiBufferMode>,
}

impl BufferSpec {
    pub fn new(name: impl Into<String>) -> Self {
        BufferSpec {
            name: name.into().to_uppercase(),
            spread: None,
            request_params: Vec::new(),
            queries: Vec::new(),
            status_fn: None,
            multi: None,
        }
    }

    pub fn with_request_params<I, S>(mut self, params: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.request_params = params
            .into_iter()
            .map(|s| s.into().to_uppercase())
            .collect();
        self
    }

    pub fn with_queries<I, S>(mut self, queries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.queries = queries
            .into_iter()
            .map(|s| s.into().to_uppercase())
            .collect();
        self
    }
}

/// A chunk type contributed by a module. This is an extension to Lisp's
/// `define-module` (Lisp modules call `chunk-type` separately).
#[derive(Debug, Clone)]
pub struct ChunkTypeSpec {
    pub name: String,
    pub parents: Vec<String>,
    pub slots: Vec<String>,
    /// `(slot default)` declarations.
    pub defaults: Vec<(String, Value)>,
}

impl ChunkTypeSpec {
    pub fn new(name: impl Into<String>) -> Self {
        ChunkTypeSpec {
            name: name.into().to_uppercase(),
            parents: Vec::new(),
            slots: Vec::new(),
            defaults: Vec::new(),
        }
    }

    pub fn with_slots<I, S>(mut self, slots: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.slots = slots.into_iter().map(|s| s.into().to_uppercase()).collect();
        self
    }

    fn into_chunk_type(self) -> ChunkType {
        ChunkType {
            name: self.name,
            parents: self.parents,
            slots: self.slots,
            defaults: self
                .defaults
                .iter()
                .map(|(slot, value)| (slot.clone(), value_to_pattern(value)))
                .collect(),
        }
    }
}

/// A parameter declared by a module (`define-parameter`).
#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub name: String,
    /// Whether this module owns the parameter (rather than observing it).
    pub owner: bool,
    pub default: Value,
    pub valid_test: Option<String>,
    pub warning: String,
    pub documentation: String,
}

impl ParamSpec {
    pub fn owned(name: impl Into<String>, default: Value) -> Self {
        ParamSpec {
            name: name.into().to_uppercase(),
            owner: true,
            default,
            valid_test: None,
            warning: String::new(),
            documentation: String::new(),
        }
    }

    pub fn observed(name: impl Into<String>) -> Self {
        ParamSpec {
            name: name.into().to_uppercase(),
            owner: false,
            default: Value::Nil,
            valid_test: None,
            warning: String::new(),
            documentation: String::new(),
        }
    }
}

/// The static description of a module, available before an instance is
/// created. This is what the model loader uses to seed chunk types and slots.
#[derive(Debug, Clone)]
pub struct ModuleDescriptor {
    pub name: String,
    pub version: String,
    pub documentation: String,
    pub buffers: Vec<BufferSpec>,
    pub chunk_types: Vec<ChunkTypeSpec>,
    pub params: Vec<ParamSpec>,
    /// `:required` metadata (`use-modules`).
    pub required: Vec<String>,
    /// `:requires` metadata (`use-modules`).
    pub requires: Vec<String>,
}

impl ModuleDescriptor {
    pub fn from_module(module: &dyn Module) -> Self {
        ModuleDescriptor {
            name: module.name().to_uppercase(),
            version: module.version().to_string(),
            documentation: module.documentation().to_string(),
            buffers: module.buffers(),
            chunk_types: module.chunk_types(),
            params: module.parameters(),
            required: module.required().to_vec(),
            requires: module.requires().to_vec(),
        }
    }

    pub fn buffer_names(&self) -> Vec<String> {
        self.buffers.iter().map(|b| b.name.clone()).collect()
    }

    pub fn param_names(&self) -> Vec<String> {
        self.params
            .iter()
            .filter(|p| p.owner)
            .map(|p| p.name.clone())
            .collect()
    }
}

/// The module definitions available to a model loader (`define-module`
/// metadata, without per-engine state).
#[derive(Debug, Clone, Default)]
pub struct ModuleSet {
    descriptors: Vec<ModuleDescriptor>,
}

impl ModuleSet {
    pub fn new(descriptors: Vec<ModuleDescriptor>) -> Self {
        ModuleSet { descriptors }
    }

    pub fn descriptors(&self) -> &[ModuleDescriptor] {
        &self.descriptors
    }

    pub fn chunk_types(&self) -> Vec<ChunkType> {
        self.descriptors
            .iter()
            .flat_map(|d| d.chunk_types.iter().cloned().map(|c| c.into_chunk_type()))
            .collect()
    }

    pub fn slot_names(&self) -> Vec<String> {
        self.descriptors
            .iter()
            .flat_map(|d| d.chunk_types.iter())
            .flat_map(|c| c.slots.iter().cloned())
            .collect()
    }

    pub fn buffer_names(&self) -> Vec<String> {
        self.descriptors
            .iter()
            .flat_map(|d| d.buffers.iter().map(|b| b.name.clone()))
            .collect()
    }

    /// Request parameters and extra query names contributed by module buffers.
    pub fn request_param_names(&self) -> Vec<String> {
        self.descriptors
            .iter()
            .flat_map(|d| d.buffers.iter())
            .flat_map(|b| {
                b.request_params
                    .iter()
                    .chain(b.queries.iter())
                    .map(|s| s.to_uppercase())
            })
            .collect()
    }

    pub fn param_names(&self) -> HashSet<String> {
        self.descriptors
            .iter()
            .flat_map(|d| d.param_names())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

/// A per-engine factory for module instances.
pub trait ModuleFactory: 'static {
    /// Describe the module without creating an instance. The default creates
    /// a throw-away instance, which is fine for stateless Rust modules.
    fn describe(&self) -> ModuleDescriptor {
        let module = self.create();
        ModuleDescriptor::from_module(&*module)
    }

    fn create(&self) -> Box<dyn Module>;
}

impl<F> ModuleFactory for F
where
    F: Fn() -> Box<dyn Module> + 'static,
{
    fn create(&self) -> Box<dyn Module> {
        self()
    }
}

/// A module implementation. Only `name` is required; every hook has a no-op
/// default, mirroring Lisp's optional `define-module` callbacks.
pub trait Module: 'static {
    fn name(&self) -> &str;

    fn version(&self) -> &str {
        ""
    }

    fn documentation(&self) -> &str {
        ""
    }

    fn buffers(&self) -> Vec<BufferSpec> {
        Vec::new()
    }

    fn chunk_types(&self) -> Vec<ChunkTypeSpec> {
        Vec::new()
    }

    fn parameters(&self) -> Vec<ParamSpec> {
        Vec::new()
    }

    /// `:required` (`use-modules`).
    fn required(&self) -> &[String] {
        &[]
    }

    /// `:requires` (`use-modules`).
    fn requires(&self) -> &[String] {
        &[]
    }

    fn create(&mut self, _ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        Ok(())
    }

    fn reset(&mut self, _ctx: &mut ModuleCtx<'_>, _phase: ResetPhase) -> Result<(), ModuleError> {
        Ok(())
    }

    fn request(
        &mut self,
        _ctx: &mut ModuleCtx<'_>,
        _request: Request<'_>,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn buffer_mod(
        &mut self,
        _ctx: &mut ModuleCtx<'_>,
        _request: ModifyRequest<'_>,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    fn query(&self, _buffer: &str, _name: &str, _value: &Value) -> Option<bool> {
        None
    }

    /// Get (`value == None`) or set an owned parameter.
    fn params(&mut self, _ctx: &mut ModuleCtx<'_>, _name: &str, _value: Option<&Value>) -> Value {
        Value::Nil
    }

    fn delete(&mut self, _ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        Ok(())
    }

    fn notify_on_clear(&mut self, _ctx: &mut ModuleCtx<'_>, _buffer: &str, _chunk: Option<&str>) {}

    fn update(&mut self, _ctx: &mut ModuleCtx<'_>, _old_ms: i64, _new_ms: i64) {}

    fn warning(&mut self, _ctx: &mut ModuleCtx<'_>, _buffer: &str, _request: Request<'_>) {}

    fn search(&mut self, _ctx: &mut ModuleCtx<'_>, _buffer: &str) -> Option<Vec<String>> {
        None
    }

    fn offset(
        &mut self,
        _ctx: &mut ModuleCtx<'_>,
        _buffer: &str,
        _chunks: &[String],
    ) -> Option<Vec<f64>> {
        None
    }

    fn run_start(&mut self, _ctx: &mut ModuleCtx<'_>) {}

    fn run_end(&mut self, _ctx: &mut ModuleCtx<'_>) {}

    /// A named event scheduled by [`ModuleCtx::schedule_named`].
    fn on_event(&mut self, _ctx: &mut ModuleCtx<'_>, _event: ModuleEvent) {}
}

#[derive(Debug, Clone)]
pub struct ModuleError(pub String);

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ModuleError {}

impl From<ModuleError> for super::EngineError {
    fn from(e: ModuleError) -> Self {
        bad(e.0)
    }
}

/// A resolved request delivered to [`Module::request`].
#[derive(Debug, Clone)]
pub struct Request<'a> {
    pub buffer: &'a str,
    pub slots: &'a [(String, Value)],
    pub chunk: Option<&'a str>,
    pub var_slots: &'a [String],
}

/// A `*buffer>` modification request delivered to [`Module::buffer_mod`].
#[derive(Debug, Clone)]
pub struct ModifyRequest<'a> {
    pub buffer: &'a str,
    pub slots: &'a [(String, Value)],
}

/// A named module event (`ctx.schedule_named`).
#[derive(Debug, Clone)]
pub struct ModuleEvent {
    pub name: String,
    pub payload: Vec<Value>,
}

/// The engine operations available to a module callback.
pub struct ModuleCtx<'a> {
    engine: &'a mut super::Engine,
    module: usize,
    name: String,
}

impl<'a> ModuleCtx<'a> {
    pub(crate) fn new(engine: &'a mut super::Engine, module: usize, name: String) -> Self {
        ModuleCtx {
            engine,
            module,
            name,
        }
    }

    pub fn module_index(&self) -> usize {
        self.module
    }

    pub fn module_name(&self) -> &str {
        &self.name
    }

    pub fn time_ms(&self) -> i64 {
        self.engine.time_ms
    }

    pub fn time_secs(&self) -> f64 {
        self.engine.time_ms as f64 / 1000.0
    }

    /// Emit a trace line under this module's name.
    pub fn emit(&mut self, details: impl Into<String>) {
        self.engine.emit(&self.name, details.into());
    }

    pub fn buffer_busy(&self, buffer: &str) -> bool {
        self.engine
            .buffers
            .get(&buffer.to_uppercase())
            .is_some_and(|b| b.busy)
    }

    pub fn set_busy(&mut self, buffer: &str, busy: bool) {
        self.engine.buffer_mut(&buffer.to_uppercase()).busy = busy;
    }

    pub fn set_error(&mut self, buffer: &str, error: bool) {
        let b = self.engine.buffer_mut(&buffer.to_uppercase());
        b.busy = false;
        b.failed = error;
    }

    /// Place a chunk description into one of this module's buffers.
    pub fn set_buffer(
        &mut self,
        buffer: &str,
        ctype: &str,
        slots: Vec<(String, Value)>,
    ) -> Result<(), ModuleError> {
        let desc = ChunkDesc::Spec {
            ctype: ctype.to_uppercase(),
            slots: slots
                .iter()
                .map(|(s, v)| (s.to_uppercase(), value_to_pattern(v)))
                .collect(),
            auto_cmd: false,
            neg: Vec::new(),
            rels: Vec::new(),
        };
        self.engine
            .set_buffer_chunk_desc(buffer, &desc)
            .map_err(|e| ModuleError(e.to_string()))
    }

    pub fn clear_buffer(&mut self, buffer: &str) {
        let b = self.engine.buffer_mut(&buffer.to_uppercase());
        b.chunk = None;
        b.failed = false;
        b.modified = false;
    }

    pub fn mod_buffer(
        &mut self,
        buffer: &str,
        slots: Vec<(String, Value)>,
    ) -> Result<(), ModuleError> {
        let b = self.engine.buffer_mut(&buffer.to_uppercase());
        let chunk = b
            .chunk
            .as_mut()
            .ok_or_else(|| ModuleError(format!("MOD-BUFFER-CHUNK with empty {buffer}")))?;
        for (slot, value) in slots {
            match chunk.slots.iter_mut().find(|(k, _)| *k == slot) {
                Some((_, old)) => *old = value,
                None => chunk.slots.push((slot, value)),
            }
        }
        b.modified = true;
        self.engine.changed();
        Ok(())
    }

    /// Schedule a named callback into this module (`Module::on_event`).
    pub fn schedule_named(&mut self, delay_ms: i64, name: impl Into<String>, payload: Vec<Value>) {
        let at = self.engine.time_ms + delay_ms.max(0);
        self.engine.push(
            at,
            Prio::Num(0),
            Out::Medium,
            Event::ModuleCallback {
                module: self.module,
                name: name.into(),
                payload,
            },
        );
    }

    pub fn param(&self, name: &str) -> Option<Value> {
        self.engine
            .module_param_values
            .get(&name.to_uppercase())
            .cloned()
    }
}

impl super::Engine {
    pub(crate) fn module_index_for_buffer(&self, buffer: &str) -> Option<usize> {
        self.module_by_buffer.get(&buffer.to_uppercase()).copied()
    }

    pub(crate) fn module_index_for_param(&self, key: &str) -> Option<usize> {
        self.module_param_owner.get(&key.to_uppercase()).copied()
    }

    pub(crate) fn custom_module_names(&self) -> &[String] {
        &self.module_names
    }

    fn with_module_mut<R>(
        &mut self,
        idx: usize,
        f: impl FnOnce(&mut dyn Module, &mut ModuleCtx<'_>) -> R,
    ) -> Option<R> {
        let mut module = self.modules.get_mut(idx)?.take()?;
        let name = self.module_names.get(idx)?.clone();
        let result = {
            let mut ctx = ModuleCtx::new(self, idx, name);
            f(&mut *module, &mut ctx)
        };
        self.modules[idx] = Some(module);
        Some(result)
    }

    /// Insert module buffers/slots, call each module's `create`, and apply the
    /// model's module parameters. Called once during construction, before the
    /// construction reset.
    pub(crate) fn init_custom_modules(&mut self) -> Result<(), ModuleError> {
        self.register_module_metadata();
        self.insert_module_buffers();
        for idx in 0..self.modules.len() {
            if let Some(result) = self.with_module_mut(idx, |module, ctx| module.create(ctx)) {
                result?;
            }
        }
        // Seed the parameter cache with each module's current/default value.
        for (key, idx) in self.module_param_owner.clone() {
            if let Some(result) =
                self.with_module_mut(idx, |module, ctx| module.params(ctx, &key, None))
            {
                self.module_param_values.insert(key, result);
            }
        }
        self.apply_model_module_params()
    }

    pub(crate) fn register_module_metadata(&mut self) {
        for descriptor in self.module_descriptors.clone() {
            for buffer in &descriptor.buffers {
                for parameter in &buffer.request_params {
                    self.valid_slots.insert(parameter.to_uppercase());
                }
                for query in &buffer.queries {
                    self.valid_slots.insert(query.to_uppercase());
                }
            }
            for chunk_type in &descriptor.chunk_types {
                for slot in &chunk_type.slots {
                    self.valid_slots.insert(slot.to_uppercase());
                }
            }
        }
    }

    pub(crate) fn insert_module_buffers(&mut self) {
        for name in self
            .module_descriptors
            .iter()
            .flat_map(|d| d.buffers.iter().map(|b| b.name.to_uppercase()))
        {
            self.buffers
                .entry(name)
                .or_insert_with(super::Buffer::empty);
        }
    }

    pub(crate) fn apply_model_module_params(&mut self) -> Result<(), ModuleError> {
        let params = self.model.params.clone();
        for (key, value) in params {
            if self.module_param_owner.contains_key(&key.to_uppercase()) {
                let value = Value::from_sexp(&value).map_err(|e| ModuleError(e.to_string()))?;
                self.set_module_param(&key, &value)?;
            }
        }
        Ok(())
    }

    pub(crate) fn set_module_param(
        &mut self,
        key: &str,
        value: &Value,
    ) -> Result<Value, ModuleError> {
        let upper = key.to_uppercase();
        let Some(idx) = self.module_param_owner.get(&upper).copied() else {
            return Ok(value.clone());
        };
        let result = self
            .with_module_mut(idx, |module, ctx| module.params(ctx, &upper, Some(value)))
            .unwrap_or_else(|| value.clone());
        self.module_param_values.insert(upper, result.clone());
        Ok(result)
    }

    pub(crate) fn module_param(&self, key: &str) -> Option<Value> {
        self.module_param_values.get(&key.to_uppercase()).cloned()
    }

    pub(crate) fn module_query(&self, buffer: &str, name: &str, value: &Value) -> Option<bool> {
        let idx = self.module_index_for_buffer(buffer)?;
        let module = self.modules.get(idx)?.as_ref()?;
        module.query(&buffer.to_uppercase(), name, value)
    }

    pub(crate) fn module_request(
        &mut self,
        idx: usize,
        buffer: &str,
        spec: &super::NewChunk,
        var_slots: &[String],
    ) -> Result<(), ModuleError> {
        let (slots, chunk) = match spec {
            super::NewChunk::Named(name) => (Vec::new(), Some(name.clone())),
            super::NewChunk::Spec { slots, .. } => (slots.clone(), None),
            super::NewChunk::Instance(inst) => (inst.slots.clone(), inst.name.clone()),
        };
        let request = Request {
            buffer,
            slots: &slots,
            chunk: chunk.as_deref(),
            var_slots,
        };
        if let Some(result) =
            self.with_module_mut(idx, move |module, ctx| module.request(ctx, request))
        {
            result?;
        }
        Ok(())
    }

    pub(crate) fn module_buffer_mod(
        &mut self,
        idx: usize,
        buffer: &str,
        slots: &[(String, Value)],
    ) -> Result<(), ModuleError> {
        let request = ModifyRequest { buffer, slots };
        if let Some(result) =
            self.with_module_mut(idx, move |module, ctx| module.buffer_mod(ctx, request))
        {
            result?;
        }
        Ok(())
    }

    pub(crate) fn module_notify_clear(&mut self, buffer: &str, chunk: Option<&str>) {
        let Some(idx) = self.module_index_for_buffer(buffer) else {
            return;
        };
        let buffer = buffer.to_string();
        let chunk = chunk.map(str::to_string);
        let _ = self.with_module_mut(idx, move |module, ctx| {
            module.notify_on_clear(ctx, &buffer, chunk.as_deref())
        });
    }

    pub(crate) fn module_on_event(
        &mut self,
        idx: usize,
        name: String,
        payload: Vec<Value>,
    ) -> Result<(), ModuleError> {
        let event = ModuleEvent { name, payload };
        let _ = self.with_module_mut(idx, move |module, ctx| module.on_event(ctx, event));
        Ok(())
    }

    pub(crate) fn module_reset(&mut self, phase: ResetPhase) -> Result<(), ModuleError> {
        for idx in 0..self.modules.len() {
            if let Some(result) = self.with_module_mut(idx, |module, ctx| module.reset(ctx, phase))
            {
                result?;
            }
        }
        Ok(())
    }

    pub(crate) fn module_update(&mut self, old_ms: i64, new_ms: i64) {
        if old_ms == new_ms || self.modules.is_empty() {
            return;
        }
        for idx in 0..self.modules.len() {
            let _ = self.with_module_mut(idx, |module, ctx| module.update(ctx, old_ms, new_ms));
        }
    }

    pub(crate) fn module_run_start(&mut self) {
        for idx in 0..self.modules.len() {
            let _ = self.with_module_mut(idx, |module, ctx| module.run_start(ctx));
        }
    }

    pub(crate) fn module_run_end(&mut self) {
        for idx in 0..self.modules.len() {
            let _ = self.with_module_mut(idx, |module, ctx| module.run_end(ctx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::load_models_from_str_with_modules;

    struct Counter {
        step: i64,
    }

    impl Module for Counter {
        fn name(&self) -> &str {
            "COUNTER"
        }

        fn version(&self) -> &str {
            "0.1"
        }

        fn documentation(&self) -> &str {
            "test counter"
        }

        fn buffers(&self) -> Vec<BufferSpec> {
            vec![BufferSpec::new("counter").with_request_params(["inc"])]
        }

        fn chunk_types(&self) -> Vec<ChunkTypeSpec> {
            vec![ChunkTypeSpec::new("counter").with_slots(["value"])]
        }

        fn parameters(&self) -> Vec<ParamSpec> {
            vec![ParamSpec::owned(":counter-step", Value::Int(1))]
        }

        fn request(
            &mut self,
            ctx: &mut ModuleCtx<'_>,
            request: Request<'_>,
        ) -> Result<(), ModuleError> {
            if request.buffer == "COUNTER" {
                let inc = request
                    .slots
                    .iter()
                    .find(|(slot, _)| slot == "INC")
                    .and_then(|(_, value)| match value {
                        Value::Int(n) => Some(*n),
                        _ => None,
                    })
                    .unwrap_or(1);
                ctx.emit(format!("REQUEST inc {inc}"));
                ctx.set_busy("COUNTER", true);
                ctx.schedule_named(50, "done", vec![Value::Int(inc * self.step)]);
            }
            Ok(())
        }

        fn on_event(&mut self, ctx: &mut ModuleCtx<'_>, event: ModuleEvent) {
            if event.name == "done" {
                let value = match event.payload.first() {
                    Some(Value::Int(n)) => *n,
                    _ => 0,
                };
                ctx.set_buffer(
                    "COUNTER",
                    "counter",
                    vec![("VALUE".into(), Value::Int(value))],
                )
                .unwrap();
                ctx.set_busy("COUNTER", false);
                ctx.emit(format!("DONE {value}"));
            }
        }

        fn query(&self, buffer: &str, name: &str, value: &Value) -> Option<bool> {
            if buffer == "COUNTER" && name == "STATE" && value == &Value::Sym("FREE".into()) {
                return Some(true);
            }
            None
        }

        fn params(&mut self, _ctx: &mut ModuleCtx<'_>, name: &str, value: Option<&Value>) -> Value {
            if name == ":COUNTER-STEP" {
                if let Some(Value::Int(n)) = value {
                    self.step = *n;
                }
                return Value::Int(self.step);
            }
            Value::Nil
        }
    }

    fn factories() -> Vec<Box<dyn ModuleFactory>> {
        vec![Box::new(|| {
            Box::new(Counter { step: 1 }) as Box<dyn Module>
        })]
    }

    const MODEL: &str = r#"
(clear-all)
(define-model custom
  (chunk-type goal state)
  (add-dm (start isa goal state start))
  (goal-focus start)
  (p fire
     =goal>
       state start
     ==>
       +counter> inc 2
       =goal>
       state done))
"#;

    #[test]
    fn custom_module_handles_requests_and_events() {
        let factories = factories();
        let modules = ModuleSet::new(factories.iter().map(|factory| factory.describe()).collect());
        let model = load_models_from_str_with_modules(MODEL, &modules)
            .expect("model loads")
            .pop()
            .expect("one model");
        let mut engine = super::super::Engine::with_module_factories(
            model,
            (100, 0),
            2.0,
            super::super::DeviceSetup::default(),
            crate::trace::MODULE_WIDTH,
            factories,
        )
        .expect("engine builds");
        engine.run_secs(1.0).expect("run");
        let trace = engine.output();
        assert!(trace.contains("REQUEST inc 2"), "{trace}");
        assert!(trace.contains("DONE 2"), "{trace}");
        assert_eq!(
            engine.buffer_slot_value("COUNTER", "VALUE").as_deref(),
            Some("2"),
            "{trace}"
        );
    }

    #[test]
    fn custom_module_parameter_is_routed() {
        let factories = factories();
        let modules = ModuleSet::new(factories.iter().map(|factory| factory.describe()).collect());
        let model = load_models_from_str_with_modules(MODEL, &modules)
            .expect("model loads")
            .pop()
            .expect("one model");
        let mut engine = super::super::Engine::with_module_factories(
            model,
            (100, 0),
            2.0,
            super::super::DeviceSetup::default(),
            crate::trace::MODULE_WIDTH,
            factories,
        )
        .expect("engine builds");
        engine
            .apply_sgp(":COUNTER-STEP", &crate::sexpr::Sexp::Int(3))
            .expect("sgp");
        assert_eq!(engine.get_param(":counter-step").as_deref(), Some("3"));
    }
}

fn value_to_pattern(value: &Value) -> Pattern {
    match value {
        Value::Sym(s) => Pattern::Sym(s.clone()),
        Value::Int(i) => Pattern::Int(*i),
        Value::Num(x) => Pattern::Num(*x),
        Value::Str(s) => Pattern::Str(s.clone()),
        Value::Nil => Pattern::Nil,
        Value::List(values) => Pattern::List(values.iter().map(value_to_pattern).collect()),
    }
}
