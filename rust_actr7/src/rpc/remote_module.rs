//! Remote (`define-module`) module support.
//!
//! The Python client sends a module's buffer/parameter descriptors and the
//! names of the client-owned commands that implement its callbacks. The
//! [`RemoteModuleFactory`] captures those descriptors plus a [`ModuleHost`];
//! each engine gets its own [`RemoteModule`] instance.
//!
//! M3 only performs the synchronous data callbacks (`create`, `query`,
//! `params`, `search`, `offset`). Side-effect callbacks are stored for the
//! deferred bridge in Milestone 4.

use std::cell::RefCell;
use std::rc::Rc;

use crate::engine::module::{
    BufferSpec, Module, ModuleCtx, ModuleDescriptor, ModuleError, ModuleFactory, MultiBufferMode,
    ParamSpec, ResetPhase,
};
use crate::engine::Value;
use crate::json::Json;

/// A synchronous host callback used by a remote module.
///
/// `model` is the model instance name. Implementations must not re-enter the
/// engine: they run while the session/engine is borrowed.
pub trait ModuleHost {
    fn invoke(&self, command: &str, model: &str, args: Vec<Json>) -> Result<Vec<Json>, String>;
}

#[derive(Debug, Clone, Default)]
pub struct RemoteCallbacks {
    pub creation: Option<String>,
    pub reset: [Option<String>; 3],
    pub query: Option<String>,
    pub request: Option<String>,
    pub buffer_mod: Option<String>,
    pub params: Option<String>,
    pub delete: Option<String>,
    pub notify_on_clear: Option<String>,
    pub update: Option<String>,
    pub warning: Option<String>,
    pub search: Option<String>,
    pub offset: Option<String>,
    pub run_start: Option<String>,
    pub run_end: Option<String>,
}

#[derive(Debug)]
struct ModuleDefinition {
    version: String,
    documentation: String,
    required: Vec<String>,
    requires: Vec<String>,
    callbacks: RemoteCallbacks,
}

/// Build a factory from the Python client's `define-module` argument shape.
pub fn define_module(
    name: &str,
    buffers: &Json,
    params: &Json,
    interface: &Json,
    host: Rc<dyn ModuleHost>,
    connection: Option<String>,
) -> Result<RemoteModuleFactory, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("define-module requires a module name".into());
    }
    let buffers = parse_buffers(buffers)?;
    let params = parse_params(params)?;
    let definition = parse_interface(interface)?;
    if !buffers.is_empty() && definition.callbacks.query.is_none() {
        return Err(format!(
            "Module {} has buffers and must provide a query callback.",
            name.to_uppercase()
        ));
    }
    if !params.is_empty() && definition.callbacks.params.is_none() {
        return Err(format!(
            "Module {} has parameters and must provide a params callback.",
            name.to_uppercase()
        ));
    }
    let descriptor = ModuleDescriptor {
        name: name.to_uppercase(),
        version: definition.version,
        documentation: definition.documentation,
        buffers,
        chunk_types: Vec::new(),
        params,
        required: definition.required,
        requires: definition.requires,
    };
    Ok(RemoteModuleFactory {
        descriptor,
        callbacks: definition.callbacks,
        host,
        connection,
    })
}

pub struct RemoteModuleFactory {
    descriptor: ModuleDescriptor,
    callbacks: RemoteCallbacks,
    host: Rc<dyn ModuleHost>,
    connection: Option<String>,
}

impl RemoteModuleFactory {
    pub fn descriptor(&self) -> &ModuleDescriptor {
        &self.descriptor
    }
}

impl ModuleFactory for RemoteModuleFactory {
    fn describe(&self) -> ModuleDescriptor {
        self.descriptor.clone()
    }

    fn name(&self) -> String {
        self.descriptor.name.clone()
    }

    fn connection(&self) -> Option<String> {
        self.connection.clone()
    }

    fn create(&self) -> Box<dyn Module> {
        Box::new(RemoteModule {
            descriptor: self.descriptor.clone(),
            callbacks: self.callbacks.clone(),
            host: self.host.clone(),
            connection: self.connection.clone(),
            model: RefCell::new(String::new()),
        })
    }
}

struct RemoteModule {
    descriptor: ModuleDescriptor,
    callbacks: RemoteCallbacks,
    host: Rc<dyn ModuleHost>,
    connection: Option<String>,
    /// Model instance name, captured by `create`. `query` is `&self` and has
    /// no `ModuleCtx`, so it needs the name saved here.
    model: RefCell<String>,
}

impl RemoteModule {
    fn call(
        &self,
        command: Option<&str>,
        model: &str,
        callback: &str,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, ModuleError> {
        let command = command.ok_or_else(|| {
            ModuleError(format!(
                "Module {} has no {} callback.",
                self.descriptor.name, callback
            ))
        })?;
        self.host.invoke(command, model, args).map_err(ModuleError)
    }

    fn first(&self, values: Vec<Json>) -> Option<Json> {
        values.into_iter().next()
    }

    fn enqueue(&self, ctx: &mut ModuleCtx<'_>, command: Option<&str>, args: Vec<Value>) {
        if let Some(command) = command {
            ctx.enqueue_callback(self.connection.clone(), command, args);
        }
    }
}

impl Module for RemoteModule {
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

    fn chunk_types(&self) -> Vec<crate::engine::module::ChunkTypeSpec> {
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
        *self.model.borrow_mut() = ctx.model_name().to_string();
        if let Some(creation) = self.callbacks.creation.as_deref() {
            self.call(Some(creation), ctx.model_name(), "creation", Vec::new())?;
        }
        Ok(())
    }

    fn query(&self, buffer: &str, name: &str, value: &Value) -> Option<bool> {
        let values = self
            .call(
                self.callbacks.query.as_deref(),
                &self.model.borrow(),
                "query",
                vec![
                    Json::Str(buffer.to_uppercase()),
                    Json::Str(name.to_uppercase()),
                    value_to_json(value),
                ],
            )
            .ok()?;
        match self.first(values)? {
            Json::Bool(value) => Some(value),
            Json::Null => Some(false),
            Json::Str(value) => Some(value.eq_ignore_ascii_case("t")),
            _ => Some(false),
        }
    }

    fn params(&mut self, ctx: &mut ModuleCtx<'_>, name: &str, value: Option<&Value>) -> Value {
        let Some(callback) = self.callbacks.params.as_deref() else {
            return value.cloned().unwrap_or(Value::Nil);
        };
        let param = match value {
            Some(value) => Json::Arr(vec![Json::Str(name.to_uppercase()), value_to_json(value)]),
            None => Json::Arr(vec![Json::Str(name.to_uppercase())]),
        };
        let values = self.call(Some(callback), ctx.model_name(), "params", vec![param]);
        match values.ok().and_then(|values| self.first(values)) {
            Some(value) => json_to_value(&value),
            None => Value::Nil,
        }
    }

    fn reset(&mut self, ctx: &mut ModuleCtx<'_>, phase: ResetPhase) -> Result<(), ModuleError> {
        let index = match phase {
            ResetPhase::Primary => 0,
            ResetPhase::Secondary => 1,
            ResetPhase::Tertiary => 2,
        };
        self.enqueue(ctx, self.callbacks.reset[index].as_deref(), Vec::new());
        Ok(())
    }

    fn notify_on_clear(&mut self, ctx: &mut ModuleCtx<'_>, buffer: &str, chunk: Option<&str>) {
        let chunk = chunk
            .map(|chunk| Value::Sym(chunk.to_uppercase()))
            .unwrap_or(Value::Nil);
        self.enqueue(
            ctx,
            self.callbacks.notify_on_clear.as_deref(),
            vec![Value::Sym(buffer.to_uppercase()), chunk],
        );
    }

    fn update(&mut self, ctx: &mut ModuleCtx<'_>, old_ms: i64, new_ms: i64) {
        self.enqueue(
            ctx,
            self.callbacks.update.as_deref(),
            vec![Value::Int(old_ms), Value::Int(new_ms)],
        );
    }

    fn delete(&mut self, ctx: &mut ModuleCtx<'_>) -> Result<(), ModuleError> {
        self.enqueue(ctx, self.callbacks.delete.as_deref(), Vec::new());
        Ok(())
    }

    fn run_start(&mut self, ctx: &mut ModuleCtx<'_>) {
        self.enqueue(ctx, self.callbacks.run_start.as_deref(), Vec::new());
    }

    fn run_end(&mut self, ctx: &mut ModuleCtx<'_>) {
        self.enqueue(ctx, self.callbacks.run_end.as_deref(), Vec::new());
    }

    fn search(&mut self, ctx: &mut ModuleCtx<'_>, buffer: &str) -> Option<Vec<String>> {
        let values = self
            .call(
                self.callbacks.search.as_deref(),
                ctx.model_name(),
                "search",
                vec![Json::Str(buffer.to_uppercase())],
            )
            .ok()?;
        match self.first(values)? {
            Json::Arr(items) => Some(
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect(),
            ),
            Json::Null => None,
            _ => None,
        }
    }

    fn offset(
        &mut self,
        ctx: &mut ModuleCtx<'_>,
        buffer: &str,
        chunks: &[String],
    ) -> Option<Vec<f64>> {
        let values = self
            .call(
                self.callbacks.offset.as_deref(),
                ctx.model_name(),
                "offset",
                vec![
                    Json::Str(buffer.to_uppercase()),
                    Json::Arr(chunks.iter().cloned().map(Json::Str).collect()),
                ],
            )
            .ok()?;
        match self.first(values)? {
            Json::Arr(items) => Some(
                items
                    .iter()
                    .filter_map(|item| match item {
                        Json::Int(i) => Some(*i as f64),
                        Json::Float(x) => Some(*x),
                        _ => None,
                    })
                    .collect(),
            ),
            Json::Null => None,
            _ => None,
        }
    }
}

fn parse_buffers(json: &Json) -> Result<Vec<BufferSpec>, String> {
    let items = match json {
        Json::Null => return Ok(Vec::new()),
        Json::Arr(items) => items,
        _ => return Err("Module buffer-list must be a list.".into()),
    };
    let mut buffers = Vec::new();
    for item in items {
        let spec = match item {
            Json::Str(name) => BufferSpec::new(name),
            Json::Arr(items) => parse_buffer(items)?,
            _ => {
                return Err(format!(
                    "Invalid buffer specification {item:?}; expected a string or list."
                ))
            }
        };
        if buffers
            .iter()
            .any(|b: &BufferSpec| b.name.eq_ignore_ascii_case(&spec.name))
        {
            return Err(format!("Buffer {} is listed more than once.", spec.name));
        }
        buffers.push(spec);
    }
    Ok(buffers)
}

fn parse_buffer(items: &[Json]) -> Result<BufferSpec, String> {
    let Some(name) = items.first().and_then(|item| item.as_str()) else {
        return Err("Buffer specification has no name.".into());
    };
    let mut spec = BufferSpec::new(name);
    if items.len() > 6 {
        return Err(format!(
            "Invalid buffer specification for {name}: too many items."
        ));
    }
    if let Some(second) = items.get(1).filter(|item| !matches!(item, Json::Null)) {
        match second {
            Json::Arr(pair) if pair.len() == 2 => {
                let param = pair[0]
                    .as_str()
                    .filter(|value| value.starts_with(':'))
                    .ok_or_else(|| {
                        format!("Invalid buffer spread parameter in {name}: {second:?}")
                    })?;
                let default = number(&pair[1])
                    .ok_or_else(|| format!("Buffer spread default for {name} must be a number."))?;
                spec.spread = Some((param.to_uppercase(), Some(default)));
            }
            Json::Str(param) if param.starts_with(':') => {
                spec.spread = Some((param.to_uppercase(), None));
            }
            other => {
                return Err(format!(
                    "Invalid buffer spread specification for {name}: {other:?}"
                ))
            }
        }
    }
    if let Some(requests) = items.get(2).filter(|item| !matches!(item, Json::Null)) {
        let Json::Arr(requests) = requests else {
            return Err(format!("Buffer request-params for {name} must be a list."));
        };
        spec.request_params = strings(requests, "request parameter")?;
    }
    if let Some(queries) = items.get(3).filter(|item| !matches!(item, Json::Null)) {
        let Json::Arr(queries) = queries else {
            return Err(format!("Buffer queries for {name} must be a list."));
        };
        spec.queries = strings(queries, "query name")?;
    }
    if let Some(status) = items.get(4).filter(|item| !matches!(item, Json::Null)) {
        spec.status_fn = match status {
            Json::Str(value) => Some(value.clone()),
            other => return Err(format!("Buffer status function for {name}: {other:?}")),
        };
    }
    if let Some(multi) = items.get(5).filter(|item| !matches!(item, Json::Null)) {
        let Json::Str(value) = multi else {
            return Err(format!("Buffer multi mode for {name} must be a string."));
        };
        spec.multi = Some(match value.to_ascii_lowercase().as_str() {
            ":multi" => MultiBufferMode::Multi,
            ":search" => MultiBufferMode::Search,
            ":multi-copy" => MultiBufferMode::MultiCopy,
            ":search-copy" => MultiBufferMode::SearchCopy,
            other => return Err(format!("Invalid multi-buffer mode {other:?} for {name}.")),
        });
    }
    Ok(spec)
}

fn parse_params(json: &Json) -> Result<Vec<ParamSpec>, String> {
    let items = match json {
        Json::Null => return Ok(Vec::new()),
        Json::Arr(items) => items,
        _ => return Err("Module param-list must be a list.".into()),
    };
    let mut params = Vec::new();
    for item in items {
        let Json::Arr(pair) = item else {
            return Err(format!("Invalid module parameter specification {item:?}."));
        };
        let Some(name) = pair.first().and_then(|value| value.as_str()) else {
            return Err("Module parameter has no name.".into());
        };
        let mut param = ParamSpec {
            name: name.to_uppercase(),
            owner: true,
            default: Value::Nil,
            valid_test: None,
            warning: String::new(),
            documentation: String::new(),
        };
        if let Some(options) = pair.get(1).filter(|value| !matches!(value, Json::Null)) {
            let Json::Arr(options) = options else {
                return Err(format!("Options for parameter {name} must be a list."));
            };
            for option in options {
                let Json::Arr(option) = option else {
                    return Err(format!("Invalid option for parameter {name}: {option:?}."));
                };
                let Some(key) = option.first().and_then(|value| value.as_str()) else {
                    return Err(format!("Parameter option for {name} has no name."));
                };
                let value = option.get(1).cloned().unwrap_or(Json::Null);
                match key.to_ascii_lowercase().as_str() {
                    "owner" => {
                        param.owner = matches!(value, Json::Bool(true));
                    }
                    "valid-test" => {
                        param.valid_test = value.as_str().map(str::to_string);
                    }
                    "default-value" => {
                        param.default = json_to_value(&value);
                    }
                    "warning" => {
                        param.warning = value.as_str().unwrap_or("").to_string();
                    }
                    "documentation" => {
                        param.documentation = value.as_str().unwrap_or("").to_string();
                    }
                    other => return Err(format!("Invalid option {other:?} for parameter {name}.")),
                }
            }
        }
        params.push(param);
    }
    Ok(params)
}

fn parse_interface(json: &Json) -> Result<ModuleDefinition, String> {
    let mut definition = ModuleDefinition {
        version: String::new(),
        documentation: String::new(),
        required: Vec::new(),
        requires: Vec::new(),
        callbacks: RemoteCallbacks::default(),
    };
    if matches!(json, Json::Null) {
        return Ok(definition);
    }
    let Json::Arr(options) = json else {
        return Err("Module interface must be a list.".into());
    };
    for option in options {
        let Json::Arr(option) = option else {
            return Err(format!("Invalid module interface option {option:?}."));
        };
        let Some(key) = option.first().and_then(|value| value.as_str()) else {
            return Err("Module interface option has no name.".into());
        };
        let value = option.get(1).cloned().unwrap_or(Json::Null);
        let callback = |name: &str| -> Result<Option<String>, String> {
            match &value {
                Json::Null => Ok(None),
                Json::Str(value) => Ok(Some(value.clone())),
                other => Err(format!(
                    "Module {name} callback must be a string: {other:?}"
                )),
            }
        };
        match key.to_ascii_lowercase().as_str() {
            "version" => definition.version = value.as_str().unwrap_or("").to_string(),
            "documentation" => definition.documentation = value.as_str().unwrap_or("").to_string(),
            "creation" => definition.callbacks.creation = callback("creation")?,
            "query" => definition.callbacks.query = callback("query")?,
            "request" => definition.callbacks.request = callback("request")?,
            "buffer-mod" => definition.callbacks.buffer_mod = callback("buffer-mod")?,
            "params" => definition.callbacks.params = callback("params")?,
            "delete" => definition.callbacks.delete = callback("delete")?,
            "notify-on-clear" => {
                definition.callbacks.notify_on_clear = callback("notify-on-clear")?
            }
            "update" => definition.callbacks.update = callback("update")?,
            "warning" => definition.callbacks.warning = callback("warning")?,
            "search" => definition.callbacks.search = callback("search")?,
            "offset" => definition.callbacks.offset = callback("offset")?,
            "run-start" => definition.callbacks.run_start = callback("run-start")?,
            "run-end" => definition.callbacks.run_end = callback("run-end")?,
            "reset" => definition.callbacks.reset = parse_reset(&value)?,
            "required" => definition.required = string_list(&value, "required")?,
            "requires" => definition.requires = string_list(&value, "requires")?,
            other => return Err(format!("Invalid define-module option {other:?}.")),
        }
    }
    Ok(definition)
}

fn parse_reset(value: &Json) -> Result<[Option<String>; 3], String> {
    match value {
        Json::Null => Ok([None, None, None]),
        Json::Str(command) => Ok([Some(command.clone()), None, None]),
        Json::Arr(commands) => {
            let mut reset = [None, None, None];
            for (i, command) in commands.iter().take(3).enumerate() {
                reset[i] = match command {
                    Json::Null => None,
                    Json::Str(command) => Some(command.clone()),
                    other => return Err(format!("Invalid module reset callback {other:?}.")),
                };
            }
            Ok(reset)
        }
        other => Err(format!("Invalid module reset callback {other:?}.")),
    }
}

fn strings(items: &[Json], kind: &str) -> Result<Vec<String>, String> {
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(|value| value.to_uppercase())
                .ok_or_else(|| format!("Invalid {kind} {item:?}."))
        })
        .collect()
}

fn string_list(value: &Json, kind: &str) -> Result<Vec<String>, String> {
    match value {
        Json::Null => Ok(Vec::new()),
        Json::Arr(items) => strings(items, kind),
        _ => Ok(Vec::new()),
    }
}

pub fn value_to_json(value: &Value) -> Json {
    match value {
        Value::Sym(value) => Json::Str(value.clone()),
        Value::Int(value) => Json::Int(*value),
        Value::Num(value) => Json::Float(*value),
        Value::Str(value) => Json::Str(value.clone()),
        Value::Nil => Json::Null,
        Value::List(values) => Json::Arr(values.iter().map(value_to_json).collect()),
    }
}

pub fn json_to_value(value: &Json) -> Value {
    match value {
        Json::Null => Value::Nil,
        Json::Bool(true) => Value::Sym("T".into()),
        Json::Bool(false) => Value::Nil,
        Json::Int(value) => Value::Int(*value),
        Json::Float(value) => Value::Num(*value),
        Json::Str(value) => Value::Sym(value.to_uppercase()),
        Json::Arr(values) => Value::List(values.iter().map(json_to_value).collect()),
        Json::Obj(_) => Value::Nil,
    }
}

fn number(value: &Json) -> Option<f64> {
    match value {
        Json::Int(value) => Some(*value as f64),
        Json::Float(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Host {
        calls: RefCell<Vec<(String, String, Vec<Json>)>>,
        results: RefCell<Vec<Result<Vec<Json>, String>>>,
    }

    impl ModuleHost for Host {
        fn invoke(&self, command: &str, model: &str, args: Vec<Json>) -> Result<Vec<Json>, String> {
            self.calls
                .borrow_mut()
                .push((command.to_string(), model.to_string(), args.clone()));
            if self.results.borrow().is_empty() {
                Ok(vec![Json::Null])
            } else {
                self.results.borrow_mut().remove(0)
            }
        }
    }

    fn goal_interface() -> Json {
        crate::json::parse(
            r#"[
                ["version","1.1P"],
                ["documentation","Replaced normal goal module."],
                ["creation","create-pgoal"],
                ["delete","delete-pgoal"],
                ["query","query-pgoal"],
                ["request","request-pgoal"],
                ["buffer-mod","bmod-pgoal"],
                ["reset",[null,"reset-pgoal"]]
            ]"#,
        )
        .unwrap()
    }

    fn declarative_params() -> Json {
        crate::json::parse(
            r#"[
                [":lf",[
                    ["owner",true],
                    ["valid-test","nonneg"],
                    ["default-value",1.0],
                    ["warning","non-negative number"],
                    ["documentation","Latency Factor"]
                ]],
                [":esc",[["owner",false]]]
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_goal_module_shape() {
        let buffers = crate::json::parse(r#"[["goal",[":ga",0]]]"#).unwrap();
        let factory = define_module(
            "goal",
            &buffers,
            &Json::Null,
            &goal_interface(),
            Rc::new(Host::default()),
            Some("connection-1".into()),
        )
        .unwrap();

        let descriptor = factory.descriptor();
        assert_eq!(descriptor.name, "GOAL");
        assert_eq!(descriptor.version, "1.1P");
        assert_eq!(descriptor.buffers.len(), 1);
        assert_eq!(descriptor.buffers[0].name, "GOAL");
        assert_eq!(
            descriptor.buffers[0].spread,
            Some((":GA".to_string(), Some(0.0)))
        );
        assert_eq!(factory.callbacks.query.as_deref(), Some("query-pgoal"));
        assert_eq!(factory.callbacks.reset[1].as_deref(), Some("reset-pgoal"));
        assert_eq!(factory.connection().as_deref(), Some("connection-1"));
    }

    #[test]
    fn parses_declarative_module_shape() {
        let buffers = crate::json::parse(r#"[["retrieval"]]"#).unwrap();
        let interface = crate::json::parse(
            r#"[
                ["version","1.1P"],
                ["documentation","Simple remote declarative module."],
                ["creation","create-pDM"],
                ["delete","delete-pDM"],
                ["query","query-pDM"],
                ["request","request-pDM"],
                ["notify-on-clear","cleared-pDM"],
                ["reset","reset-pDM"],
                ["params","params-pDM"]
            ]"#,
        )
        .unwrap();
        let factory = define_module(
            "declarative",
            &buffers,
            &declarative_params(),
            &interface,
            Rc::new(Host::default()),
            None,
        )
        .unwrap();

        let descriptor = factory.descriptor();
        assert_eq!(descriptor.name, "DECLARATIVE");
        assert_eq!(descriptor.buffers[0].name, "RETRIEVAL");
        assert_eq!(descriptor.params.len(), 2);
        let lf = descriptor
            .params
            .iter()
            .find(|param| param.name == ":LF")
            .unwrap();
        assert!(lf.owner);
        assert_eq!(lf.default, Value::Num(1.0));
        assert_eq!(lf.valid_test.as_deref(), Some("nonneg"));
        let esc = descriptor
            .params
            .iter()
            .find(|param| param.name == ":ESC")
            .unwrap();
        assert!(!esc.owner);
        assert_eq!(factory.callbacks.reset[0].as_deref(), Some("reset-pDM"));
    }

    #[test]
    fn rejects_buffers_without_query_callback() {
        let buffers = crate::json::parse(r#"[["probe"]]"#).unwrap();
        let error = define_module(
            "probe",
            &buffers,
            &Json::Null,
            &Json::Null,
            Rc::new(Host::default()),
            None,
        )
        .err()
        .unwrap();
        assert!(error.contains("must provide a query callback"), "{error}");
    }
}
