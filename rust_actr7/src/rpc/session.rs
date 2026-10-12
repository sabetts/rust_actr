//! RPC session: the command registry plus driving the Rust engine in response
//! to `evaluate <name> <args>` calls from a client.
//!
//! The registry mirrors ACT-R's `dispatcher-command-table`
//! (`framework/dispatcher.lisp:816`): a name maps to a handler plus
//! before/after/simple monitors. Built-in handlers are implemented here; a
//! command the *client* owns (`actr.add_command`) is a `Remote` entry the
//! server invokes by sending an `evaluate` callback back over the wire.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::engine::{DeviceSetup, Engine, EngineError};
use crate::json::Json;
use crate::model::{self, load_models, ChunkDesc, Model, UserDefun};
use crate::sexpr::Sexp;

/// Where a command's underlying function lives.
#[derive(Debug, Clone, PartialEq)]
pub enum Owner {
    /// Implemented in the Rust engine.
    Builtin,
    /// No underlying function (a pure monitor / placeholder).
    None,
    /// Owned by a remote client; the server calls back into it.
    Remote { local_name: Option<String> },
}

/// A registered command (`add-command`, `framework/dispatcher.lisp:957`).
#[derive(Debug, Clone)]
pub struct Command {
    pub owner: Owner,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub simple: Vec<String>,
    /// The connection name that added it (`add`, `set-name`), if any.
    pub connection: Option<String>,
}

impl Command {
    fn builtin() -> Command {
        Command {
            owner: Owner::Builtin,
            before: Vec::new(),
            after: Vec::new(),
            simple: Vec::new(),
            connection: None,
        }
    }
}

/// The names the Rust engine implements (`evaluate` dispatches to
/// [`Session::invoke`]). Mirrors the client stubs in
/// `tutorial/lisp/remote-actr-connection.lisp` as they are implemented.
pub const BUILTINS: &[&str] = &[
    "run",
    "run-full-time",
    "run-until-time",
    "run-n-events",
    "reset",
    "reload",
    "load-act-r-model",
    "load-act-r-code",
    "goal-focus",
    "mod-focus",
    "set-parameter-value",
    "get-parameter-value",
    "set-system-parameter-value",
    "get-system-parameter-value",
    "current-model",
    "set-current-model",
    "new-symbol",
    "mp-time",
    "mp-time-ms",
    "model-output",
    "command-output",
    "print-warning",
    "act-r-output",
    "buffer-chunk",
    "buffer-status",
    "buffer-read",
    "clear-buffer",
    "set-buffer-chunk",
    "schedule-event-relative",
    "schedule-event-now",
    "schedule-event",
    "schedule-set-buffer-chunk",
    "schedule-mod-buffer-chunk",
    "schedule-event-after-module",
    "add-dm",
    "add-dm-fct",
    "define-chunks",
    "define-chunks-fct",
    "define-chunk-spec",
    "dm",
    "sdm",
    "pprint-chunks",
    "pprint-chunks-fct",
    "whynot",
    "mp-show-queue",
    "chunk-p",
    "chunk-copied-from",
    "chunk-slot-value",
    "set-chunk-slot-value",
    "copy-chunk",
    "delete-chunk",
    "purge-chunk",
    "act-r-random",
    "permute-list",
    "get-time",
    "release-chunk-spec-id",
    "add-command",
    "remove-command",
    "monitor-command",
    "remove-command-monitor",
    "check",
    // The four trace streams (`framework/printing.lisp`); their monitors are
    // what actually receive output (the Rust engine already formatted it).
    "model-trace",
    "command-trace",
    "warning-trace",
    "general-trace",
];

/// A remote (client-owned) command the server can invoke by name.
pub trait Remote {
    fn call(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String>;
}

/// A live RPC session: loaded model(s) plus the running engine.
pub struct Session {
    pub root: PathBuf,
    model_files: Vec<String>,
    defs: Vec<Model>,
    user_fns: std::collections::HashMap<String, UserDefun>,
    commands: BTreeMap<String, Command>,
    engine: Option<Engine>,
    current_model: Option<String>,
    /// Trace lines not yet streamed to `model-trace` monitors.
    pending_trace: Vec<String>,
    trace_pos: usize,
    seed: (u64, u64),
    /// `(new-symbol …)` counter, as in ACT-R's `new-symbol`.
    symbol_count: u64,
}

impl Session {
    /// A fresh session rooted at an ACT-R checkout (`actr7.x`).
    pub fn new(root: PathBuf, seed: (u64, u64)) -> Session {
        let mut commands = BTreeMap::new();
        for name in BUILTINS {
            commands.insert((*name).to_string(), Command::builtin());
        }
        Session {
            root,
            model_files: Vec::new(),
            defs: Vec::new(),
            user_fns: Default::default(),
            commands,
            engine: None,
            current_model: None,
            pending_trace: Vec::new(),
            trace_pos: 0,
            seed,
            symbol_count: 0,
        }
    }

    /// The command registry (for `check`/`list-commands`/`add`/`remove`).
    pub fn commands(&self) -> &BTreeMap<String, Command> {
        &self.commands
    }

    /// Register a client-owned command.
    pub fn add_remote(
        &mut self,
        name: &str,
        local_name: Option<String>,
        connection: Option<String>,
    ) {
        let c = Command {
            owner: Owner::Remote { local_name },
            before: Vec::new(),
            after: Vec::new(),
            simple: Vec::new(),
            connection,
        };
        self.commands.insert(name.to_string(), c);
    }

    /// Remove a command (only client-added ones may be removed).
    pub fn remove(&mut self, name: &str) -> bool {
        match self.commands.get(name) {
            Some(c) if c.owner != Owner::Builtin => {
                self.commands.remove(name);
                true
            }
            _ => false,
        }
    }

    /// Attach a monitor (`monitor`, `framework/dispatcher.lisp:1152`).
    pub fn add_monitor(&mut self, command: &str, monitor: &str, when: MonitorWhen) {
        if let Some(c) = self.commands.get_mut(command) {
            let list = match when {
                MonitorWhen::Before => &mut c.before,
                MonitorWhen::After => &mut c.after,
                MonitorWhen::Simple => &mut c.simple,
            };
            if !list.iter().any(|m| m == monitor) {
                list.push(monitor.to_string());
            }
        }
    }

    /// Detach a monitor (`remove-monitor`).
    pub fn remove_monitor(&mut self, command: &str, monitor: &str) {
        if let Some(c) = self.commands.get_mut(command) {
            for list in [&mut c.before, &mut c.after, &mut c.simple] {
                list.retain(|m| m != monitor);
            }
        }
    }

    /// Whether a command exists (`check`).
    pub fn has_command(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    /// Take the trace lines emitted since the last call (the RPC server
    /// streams these to the `model-trace` monitors).
    pub fn take_trace(&mut self) -> Vec<String> {
        if let Some(eng) = &self.engine {
            let pos = self.trace_pos;
            let lines = eng.out_since(pos);
            self.trace_pos = eng.out_len();
            lines
        } else {
            Vec::new()
        }
    }

    pub fn current_model(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    pub fn set_current_model(&mut self, name: Option<String>) {
        self.current_model = name;
    }

    /// `evaluate <name> <args>`: run the underlying handler plus its
    /// before/after/simple monitors (`execute-command`,
    /// `framework/dispatcher.lisp:1076`).
    pub fn evaluate(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: &[Json],
        remote: &mut dyn Remote,
    ) -> Result<Vec<Json>, String> {
        let c = self.commands.get(name).cloned().ok_or_else(|| {
            format!("Command {name:?} not found in the table and cannot be executed.")
        })?;
        for m in c.before.clone() {
            if let Err(e) = self.eval_command(&m, model, args, remote) {
                self.warn(&format!(
                    "Error while calling {m:?} as a before monitor for {name:?}: {e}"
                ));
            }
        }
        let result = self.underlying(&c, name, model, args, remote);
        let (success, values) = match &result {
            Ok(v) => (true, Some(v.clone())),
            Err(_) => (false, None),
        };
        for m in c.after.clone() {
            let mut a = vec![Json::Str(name.to_string()), Json::Arr(args.to_vec())];
            a.push(Json::Bool(success));
            a.push(values.clone().map(Json::Arr).unwrap_or(Json::Null));
            if let Err(e) = self.eval_command(&m, model, &a, remote) {
                self.warn(&format!(
                    "Error while calling {m:?} as an after monitor for {name:?}: {e}"
                ));
            }
        }
        for m in c.simple.clone() {
            if let Err(e) = self.eval_command(&m, model, args, remote) {
                self.warn(&format!(
                    "Error while calling {m:?} as a monitor for {name:?}: {e}"
                ));
            }
        }
        result
    }

    /// Invoke a command's underlying function without its own monitors
    /// (`evaluate-command`).
    fn eval_command(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: &[Json],
        remote: &mut dyn Remote,
    ) -> Result<Vec<Json>, String> {
        let c = self.commands.get(name).cloned().ok_or_else(|| {
            format!("Command {name:?} not found in the table and cannot be executed.")
        })?;
        self.underlying(&c, name, model, args, remote)
    }

    fn underlying(
        &mut self,
        c: &Command,
        name: &str,
        model: Option<&str>,
        args: &[Json],
        remote: &mut dyn Remote,
    ) -> Result<Vec<Json>, String> {
        match &c.owner {
            Owner::Builtin => self.invoke(name, args),
            Owner::None => Ok(vec![Json::Bool(true)]),
            Owner::Remote { local_name } => {
                let call = local_name.clone().unwrap_or_else(|| name.to_string());
                remote.call(&call, model, args.to_vec())
            }
        }
    }

    /// Route a trace line to a trace command's monitors
    /// (`model-trace`/`command-trace`/…). The engine already formatted it, so
    /// the underlying function is a no-op.
    pub fn deliver_trace(&mut self, line: &str, remote: &mut dyn Remote) {
        for command in [
            "model-trace",
            "command-trace",
            "warning-trace",
            "general-trace",
        ] {
            let Some(c) = self.commands.get(command).cloned() else {
                continue;
            };
            if c.before.is_empty() && c.after.is_empty() && c.simple.is_empty() {
                continue;
            }
            let args = [Json::Str(line.to_string())];
            let _ = self.evaluate(command, None, &args, remote);
        }
    }

    /// A warning that is not itself part of the trace.
    fn warn(&self, _msg: &str) {}

    /// The underlying handler for a built-in command.
    fn invoke(&mut self, name: &str, args: &[Json]) -> Result<Vec<Json>, String> {
        match name {
            "new-symbol" => {
                self.symbol_count += 1;
                let base = args.first().and_then(|a| a.as_str()).unwrap_or("SYMBOL");
                Ok(vec![Json::Str(format!("{base}{}", self.symbol_count))])
            }
            "current-model" => Ok(vec![self
                .current_model
                .clone()
                .map(Json::Str)
                .unwrap_or(Json::Null)]),
            "set-current-model" => {
                self.current_model = args.first().and_then(|a| a.as_str()).map(str::to_string);
                Ok(vec![Json::Bool(true)])
            }
            "load-act-r-model" | "load-act-r-code" => {
                let logical = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("load-act-r-model requires a path")?;
                self.load(logical)?;
                Ok(vec![Json::Bool(true)])
            }
            "reset" => {
                if let Some(eng) = &mut self.engine {
                    eng.reset().map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Bool(true)])
            }
            "run" | "run-full-time" | "run-until-time" | "run-n-events" => self.run_cmd(name, args),
            "goal-focus" => {
                let desc = goal_desc(args)?;
                if let Some(eng) = &mut self.engine {
                    eng.goal_focus(&desc).map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Null])
            }
            "set-parameter-value" => {
                let key = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing parameter")?;
                let val = args.get(1).cloned().unwrap_or(Json::Null);
                let sexp = json_to_sexp(&val);
                if let Some(eng) = &mut self.engine {
                    eng.apply_sgp(key, &sexp).map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Bool(true)])
            }
            "model-output" | "command-output" | "print-warning" | "act-r-output" => {
                if let Some(engine) = &mut self.engine {
                    let text = args.iter().map(json_to_text).collect::<Vec<_>>().join(" ");
                    engine.model_output(&text);
                }
                Ok(vec![Json::Null])
            }
            "mp-time" | "mp-time-ms" | "get-time" => Ok(vec![Json::Int(0)]),
            "buffer-chunk" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.print_buffer_chunk(&names);
                }
                Ok(vec![Json::Null])
            }
            "dm" | "sdm" | "pprint-chunks" | "pprint-chunks-fct" | "whynot" | "mp-show-queue" => {
                // Output commands land in a later increment (R3); an empty
                // result keeps the protocol round-tripping for now.
                Ok(vec![Json::Null])
            }
            // The trace streams are written by the engine already; their
            // monitors are dispatched by `deliver_trace`.
            "model-trace" | "command-trace" | "warning-trace" | "general-trace" => {
                Ok(vec![Json::Null])
            }
            _ => Err(format!("Command {name:?} is not implemented yet.")),
        }
    }

    fn run_cmd(&mut self, name: &str, args: &[Json]) -> Result<Vec<Json>, String> {
        let secs = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
        let eng = self.engine.as_mut().ok_or("no model loaded")?;
        let r = match name {
            "run" => eng.run_secs(secs),
            "run-full-time" => eng.run_full_time(secs),
            "run-until-time" => eng.run_until(secs),
            "run-n-events" => eng.run_n_events(secs.max(0.0) as u64),
            _ => unreachable!(),
        };
        r.map_err(|e: EngineError| e.to_string())?;
        // `run` returns `(values seconds events break)` in ACT-R; the client
        // ignores them (`actr.run` returns the list).
        Ok(vec![Json::Int(0), Json::Int(0), Json::Null])
    }

    /// Load an ACT-R logical path (`ACT-R:tutorial;unit1;count.lisp`).
    fn load(&mut self, logical: &str) -> Result<(), String> {
        let path = logical_to_path(&self.root, logical);
        if !path.is_file() {
            return Err(format!("no such file: {}", path.display()));
        }
        if crate::file_defines_model(&path) {
            self.defs
                .extend(load_models(&path).map_err(|e| e.to_string())?);
        }
        if let Ok(src) = std::fs::read_to_string(&path) {
            self.user_fns.extend(model::parse_defuns(&src));
            for (cmd, fname) in model::parse_commands(&src) {
                if let Some(def) = self.user_fns.get(&fname).cloned() {
                    self.user_fns.insert(cmd, def);
                }
            }
        }
        self.model_files.push(logical.to_string());
        self.build_engine()
    }

    /// (Re)build the engine from the loaded models.
    fn build_engine(&mut self) -> Result<(), String> {
        let def = match self.defs.last() {
            Some(d) => d.clone(),
            None => return Ok(()),
        };
        self.current_model = Some(def.name.clone());
        let mut eng =
            Engine::new(def, self.seed, 0.0, DeviceSetup::default()).map_err(|e| e.to_string())?;
        eng.set_user_fns(self.user_fns.clone());
        self.engine = Some(eng);
        self.trace_pos = 0;
        Ok(())
    }
}

/// The monitor kind a `monitor` request installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorWhen {
    Before,
    After,
    Simple,
}

/// `logical-to-path`: `ACT-R:tutorial;unit1;count.lisp` →
/// `<root>/tutorial/unit1/count.lisp`. The host prefix up to the first `:`
/// (`ACT-R:`) is dropped, then `;` becomes the path separator.
pub fn logical_to_path(root: &std::path::Path, logical: &str) -> PathBuf {
    let after_host = match logical.find(':') {
        Some(i) => &logical[i + 1..],
        None => logical,
    };
    let rel = after_host.replace(';', std::path::MAIN_SEPARATOR_STR);
    let rel = rel.trim_start_matches(std::path::MAIN_SEPARATOR);
    root.join(rel)
}

/// JSON → the engine's `Sexp`. Strings become symbols (the Python client
/// sends names/values as strings, which ACT-R `decode-string`s).
pub fn json_to_sexp(j: &Json) -> Sexp {
    match j {
        Json::Null => Sexp::Sym("NIL".into()),
        Json::Bool(true) => Sexp::Sym("T".into()),
        Json::Bool(false) => Sexp::Sym("NIL".into()),
        Json::Int(i) => Sexp::Int(*i),
        Json::Float(x) => Sexp::Float(*x),
        Json::Str(s) => Sexp::Sym(s.to_uppercase()),
        Json::Arr(items) => Sexp::List(items.iter().map(json_to_sexp).collect()),
        Json::Obj(_) => Sexp::Sym("NIL".into()),
    }
}

/// JSON → display text (for `model-output`/`print-warning`).
pub fn json_to_text(j: &Json) -> String {
    match j {
        Json::Null => String::new(),
        Json::Bool(true) => "T".into(),
        Json::Bool(false) => "NIL".into(),
        Json::Int(i) => i.to_string(),
        Json::Float(x) => x.to_string(),
        Json::Str(s) => s.clone(),
        Json::Arr(items) => items.iter().map(json_to_text).collect::<Vec<_>>().join(" "),
        Json::Obj(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Json;

    struct NoRemote;
    impl Remote for NoRemote {
        fn call(&mut self, _n: &str, _m: Option<&str>, _a: Vec<Json>) -> Result<Vec<Json>, String> {
            Err("no remote client".into())
        }
    }

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("actr7.x")
    }

    #[test]
    fn logical_paths_resolve() {
        let p = logical_to_path(
            std::path::Path::new("/actr"),
            "ACT-R:tutorial;unit1;count.lisp",
        );
        assert_eq!(p, std::path::Path::new("/actr/tutorial/unit1/count.lisp"));
    }

    #[test]
    fn loads_runs_and_streams_a_trace() {
        let root = root();
        if !root.is_dir() {
            return; // ACT-R checkout not available
        }
        let mut s = Session::new(root, (100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str("ACT-R:tutorial;unit1;count.lisp".into())],
            &mut remote,
        )
        .unwrap();
        assert_eq!(s.current_model(), Some("COUNT"));
        s.evaluate("run", None, &[Json::Int(10)], &mut remote)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            trace.iter().any(|l| l.contains("PRODUCTION-FIRED")),
            "expected a production trace, got {trace:?}"
        );
        // The trace is drained.
        assert!(s.take_trace().is_empty());
    }

    #[test]
    fn goal_focus_accepts_a_spec_list() {
        let root = root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new(root, (100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str("ACT-R:tutorial;unit1;semantic.lisp".into())],
            &mut remote,
        )
        .unwrap();
        s.evaluate(
            "goal-focus",
            None,
            &[Json::Arr(vec![
                Json::Str("ISA".into()),
                Json::Str("is-member".into()),
                Json::Str("object".into()),
                Json::Str("canary".into()),
                Json::Str("category".into()),
                Json::Str("animal".into()),
            ])],
            &mut remote,
        )
        .unwrap();
        assert!(s.has_command("run"));
        assert!(!s.has_command("nope"));
    }
}

/// A `goal-focus` argument: a flat `["ISA","type","slot","val", …]` list (a
/// chunk spec) or a chunk name string.
fn goal_desc(args: &[Json]) -> Result<ChunkDesc, String> {
    match args.first() {
        Some(Json::Str(name)) => Ok(ChunkDesc::Named(name.to_uppercase())),
        Some(Json::Arr(items)) => {
            let forms: Vec<Sexp> = items.iter().map(json_to_sexp).collect();
            model::parse_chunk_desc(&forms).map_err(|e| e.to_string())
        }
        Some(Json::Null) | None => Err("goal-focus needs an argument".into()),
        _ => Err("bad goal-focus argument".into()),
    }
}
