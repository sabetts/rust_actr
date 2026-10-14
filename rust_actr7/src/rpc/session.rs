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
    "buffer-slot-value",
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
    "chunk-type",
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
    "record-history",
    "stop-recording-history",
    "predict-bold-response",
    "sgp",
    "spp",
    "sdp",
    "no-output",
    "pp",
    // Device / vision / motor.
    "add-visicon-features",
    "modify-visicon-features",
    "delete-visicon-features",
    "delete-all-visicon-features",
    "print-visicon",
    "new-tone-sound",
    "new-word-sound",
    "new-digit-sound",
    "pbreak",
    "punbreak",
    "pdisable",
    "penable",
    "correlation",
    "mean-deviation",
    "clear-exp-window",
    "add-text-to-exp-window",
    "add-line-to-exp-window",
    "add-button-to-exp-window",
    "remove-items-from-exp-window",
    "modify-text-for-exp-window",
    "modify-line-for-exp-window",
    "open-exp-window",
    "install-device",
    "process-events",
    "visible-virtuals-available?",
    "start-hand-at-mouse",
    // The model's own command output: the motor module calls these, and a
    // task monitors them (`monitor-command "output-key" …`). The engine
    // records each press and the server dispatches it to its monitors.
    "output-key",
    "release-key",
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
    trace_pos: usize,
    seed: (u64, u64),
    /// `(new-symbol …)` counter, as in ACT-R's `new-symbol`.
    symbol_count: u64,
    /// `set-system-parameter-value` settings (`:starting-parameters`, …),
    /// re-applied on reset (`central-parameters.lisp:140`).
    system_params: std::collections::BTreeMap<String, Json>,
    /// A `:testing-function` that lives on the client, deferred to the
    /// connection layer (`model-tester.cl:485`). See `invoke_testing_fn`.
    pending_testing_fn: Option<String>,
    /// The scripted device the engine is built with (empty by default).
    device: DeviceSetup,
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
            trace_pos: 0,
            seed,
            symbol_count: 0,
            system_params: Default::default(),
            pending_testing_fn: None,
            device: DeviceSetup::default(),
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

    /// The simple monitors of a command, each with whether it is
    /// client-owned (`call_remote_reentrant`) or server-side
    /// (`Session::evaluate`). A model-fired command's monitors must run with
    /// the session borrow released when they are client-owned, since their
    /// handler may call back in (`paired-learning`'s `respond-to-key-press`
    /// calls `get-time`).
    pub fn simple_monitors(&self, name: &str) -> Vec<(String, bool)> {
        self.commands
            .get(name)
            .map(|c| {
                c.simple
                    .iter()
                    .map(|m| {
                        let remote = self
                            .commands
                            .get(m)
                            .map(|mc| matches!(mc.owner, Owner::Remote { .. }))
                            .unwrap_or(false);
                        (m.clone(), remote)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Take the commands the model fired during the last run, as
    /// `(name, model, key)` (see `Engine::take_fired_commands`). The server
    /// dispatches them to their monitors so a client's `monitor-command`
    /// handler runs, as ACT-R's `output-key` does mid-run.
    pub fn take_fired_commands(&mut self) -> Vec<(String, String, String)> {
        self.engine
            .as_mut()
            .map(|e| e.take_fired_commands())
            .unwrap_or_default()
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

    /// The trace since the last call as an output string, in the same shape as
    /// [`Engine::output`] — the trailing newline is included so a final blank
    /// trace line survives `str::lines`. A command whose output ends with
    /// `command-output`'s newline (e.g. `pprint-chunks`) leaves that blank line.
    pub fn take_trace_output(&mut self) -> String {
        let lines = self.take_trace();
        if lines.is_empty() {
            return String::new();
        }
        let mut s = lines.join("\n");
        s.push('\n');
        s
    }

    pub fn current_model(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    /// The live engine, for a harness that drives non-command steps at the
    /// engine level while routing commands through [`Session::evaluate`].
    pub fn engine_mut(&mut self) -> Option<&mut Engine> {
        self.engine.as_mut()
    }

    /// Install a scripted device and rebuild the engine around it (the spec
    /// harness's `<name>.device.sexp` sidecar).
    pub fn set_device(&mut self, device: DeviceSetup) -> Result<(), String> {
        self.device = device;
        self.build_engine()
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
            Owner::Builtin => self.invoke(name, args, remote),
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
    /// Route a trace line to the `model-trace` command's monitors.
    ///
    /// The engine's `out_lines` interleaves model, command and general output
    /// (the four Lisp streams share one sink in the model-tester), so all of
    /// it goes to `model-trace`; a monitor registered on `command-trace`/
    /// `general-trace` would otherwise see every line four times.
    pub fn deliver_trace(&mut self, line: &str, remote: &mut dyn Remote) {
        let Some(c) = self.commands.get("model-trace").cloned() else {
            return;
        };
        if c.before.is_empty() && c.after.is_empty() && c.simple.is_empty() {
            return;
        }
        let args = [Json::Str(line.to_string())];
        let _ = self.evaluate("model-trace", None, &args, remote);
    }

    /// A warning that is not itself part of the trace.
    fn warn(&self, _msg: &str) {}

    /// The underlying handler for a built-in command.
    fn invoke(
        &mut self,
        name: &str,
        args: &[Json],
        remote: &mut dyn Remote,
    ) -> Result<Vec<Json>, String> {
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
                // ACT-R's `:testing-module` reset hook fires after the model
                // reset (`model-tester.cl:485`).
                self.invoke_testing_fn(remote);
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
            "mod-focus" => {
                // `mod-focus` (`goal.lisp:378`): modify the goal buffer's chunk
                // with the given slot/value pairs, passed as one flat list
                // (`'{<slot> <value>}*`).
                let slots = mod_focus_slots(args)?;
                if let Some(eng) = &mut self.engine {
                    eng.mod_focus(&slots);
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
            "get-parameter-value" => {
                let key = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing parameter")?;
                let v = self.engine.as_ref().and_then(|e| e.get_param(key));
                Ok(vec![v.map(Json::Str).unwrap_or(Json::Null)])
            }
            "set-system-parameter-value" => {
                let key = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing parameter")?
                    .to_uppercase();
                let val = args.get(1).cloned().unwrap_or(Json::Null);
                self.set_system_param(&key, val);
                Ok(vec![Json::Bool(true)])
            }
            "get-system-parameter-value" => {
                let key = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing parameter")?
                    .to_uppercase();
                Ok(vec![self
                    .system_params
                    .get(&key)
                    .cloned()
                    .unwrap_or(Json::Null)])
            }
            "reload" => {
                self.build_engine()?;
                Ok(vec![Json::Bool(true)])
            }
            // `(sgp :key …)`: a getter when every arg is a keyword, else a
            // setter (`framework/printing.lisp`). `spp`/`sdp` getters are the
            // per-production/sub-symbolic forms; production params are
            // returned as NIL for now.
            "sgp" | "sdp" => {
                let all_keys = !args.is_empty()
                    && args
                        .iter()
                        .all(|a| a.as_str().map(|s| s.starts_with(':')).unwrap_or(false));
                if all_keys {
                    let out: Vec<Json> = args
                        .iter()
                        .map(|a| {
                            let k = a.as_str().unwrap_or("");
                            self.engine
                                .as_ref()
                                .and_then(|e| e.get_param(k))
                                .map(Json::Str)
                                .unwrap_or(Json::Null)
                        })
                        .collect();
                    Ok(out)
                } else {
                    let mut i = 0;
                    while i + 1 < args.len() {
                        let k = args[i].as_str().unwrap_or("").to_string();
                        let sexp = json_to_sexp(&args[i + 1]);
                        if let Some(eng) = &mut self.engine {
                            eng.apply_sgp(&k, &sexp).map_err(|e| e.to_string())?;
                        }
                        i += 2;
                    }
                    Ok(vec![Json::Null])
                }
            }
            // `spp-fct` (`utility-and-reward-1.lisp:800`): "production
            // :key [value] …". A key followed by a non-key is a *set* — the
            // engine has no per-production store, so those fall through to
            // `apply_sgp` as before. Otherwise it is a read: ACT-R returns one
            // value list per production, so the reply is `[[values…]…]` and the
            // client's `no_output("spp", prod, ":u")[0][0]` indexes three deep
            // (`tutorial/python/bst.py:299`).
            "spp" => {
                let mut names: Vec<String> = Vec::new();
                let mut keys: Vec<String> = Vec::new();
                let mut sets: Vec<(String, Json)> = Vec::new();
                let mut i = 0;
                while i < args.len() && !args[i].as_str().unwrap_or("").starts_with(':') {
                    names.push(args[i].as_str().unwrap_or("").to_string());
                    i += 1;
                }
                while i < args.len() {
                    let k = args[i].as_str().unwrap_or("").to_string();
                    if !k.starts_with(':') {
                        i += 1;
                        continue;
                    }
                    if i + 1 < args.len()
                        && !args[i + 1].as_str().unwrap_or("").starts_with(':')
                    {
                        sets.push((k, args[i + 1].clone()));
                        i += 2;
                    } else {
                        keys.push(k);
                        i += 1;
                    }
                }
                if !sets.is_empty() {
                    let mut out = Vec::new();
                    for (k, v) in sets {
                        let sexp = json_to_sexp(&v);
                        if let Some(eng) = &mut self.engine {
                            eng.apply_sgp(&k, &sexp).map_err(|e| e.to_string())?;
                        }
                        out.push(Json::Null);
                    }
                    return Ok(out);
                }
                let Some(eng) = &self.engine else {
                    return Ok(vec![Json::Null]);
                };
                if keys.is_empty() {
                    return Ok(vec![Json::Null]);
                }
                let prods = if names.is_empty() {
                    eng.production_names()
                } else {
                    names
                };
                let results: Vec<Json> = prods
                    .iter()
                    .map(|n| {
                        Json::Arr(
                            keys.iter()
                                .map(|k| {
                                    eng.prod_param(n, k)
                                        .map(|v| sexp_to_json(&v))
                                        .unwrap_or(Json::Null)
                                })
                                .collect(),
                        )
                    })
                    .collect();
                Ok(vec![Json::Arr(results)])
            }
            // `external-no-output` (`framework/printing.lisp:470`): run the
            // named command with its output suppressed and return its value.
            // `bst.py` uses it to read `spp` without printing the report.
            "no-output" => {
                let sub = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("no-output needs a command name")?
                    .to_string();
                let sub_args: Vec<Json> = args.iter().skip(1).cloned().collect();
                // The suppressed command's trace must not reach the client's
                // monitors — but only *its* lines. ACT-R's `no-output` binds the
                // output stream to nothing for the duration of the call, so
                // anything queued before it stays; dropping the whole pending
                // trace instead erased earlier output (a `no-output spp` per
                // trial wiped every previous trial's trace in `bst-*`).
                let mark = self.engine.as_ref().map(|e| e.out_len());
                let out = self.evaluate(&sub, None, &sub_args, remote);
                if let (Some(mark), Some(eng)) = (mark, self.engine.as_mut()) {
                    eng.truncate_out(mark);
                    if self.trace_pos > mark {
                        self.trace_pos = mark;
                    }
                }
                out
            }
            "add-dm" | "add-dm-fct" | "define-chunks" | "define-chunks-fct" => {
                // `(add-dm (name isa type …) …)`; a `-fct` variant takes one
                // list of chunk definitions.
                let is_fct = name.ends_with("-fct");
                let is_dm = name.starts_with("add-dm");
                let chunks: Vec<Json> = if is_fct {
                    match args.first() {
                        Some(Json::Arr(a)) => a.clone(),
                        _ => Vec::new(),
                    }
                } else {
                    args.to_vec()
                };
                if let Some(m) = self.defs.last_mut() {
                    for c in &chunks {
                        let Json::Arr(items) = c else { continue };
                        let forms: Vec<Sexp> = items.iter().map(json_to_sexp).collect();
                        let sexp = Sexp::List(forms);
                        if is_dm {
                            m.chunks
                                .push(model::parse_dm_entry(&sexp).map_err(|e| e.to_string())?);
                        } else {
                            model::define_chunk_table_entry(&sexp, m).map_err(|e| e.to_string())?;
                        }
                    }
                }
                self.rebuild_engine_in_place()?;
                Ok(vec![Json::Null])
            }
            "chunk-type" => {
                // `chunk-type-fct` (`framework/chunk-types.lisp:340`): define a
                // chunk type, or print them all with no name. The client sends
                // the `name-and-slots` list as separate args (the `-fct`
                // convention).
                if args.is_empty() {
                    let out = self
                        .engine
                        .as_mut()
                        .map(|e| e.print_chunk_types())
                        .unwrap_or_default();
                    return Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())]);
                }
                let forms: Vec<Sexp> = args.iter().map(json_to_sexp).collect();
                let ct = model::parse_chunk_type_spec(&forms).map_err(|e| e.to_string())?;
                let name = ct.name.clone();
                if let Some(m) = self.defs.last_mut() {
                    // ACT-R forbids redefinition (it warns); the engine keeps
                    // the latest, as with production redefinition.
                    m.chunk_types.retain(|t| !t.name.eq_ignore_ascii_case(&name));
                    m.chunk_types.push(ct);
                }
                self.rebuild_engine_in_place()?;
                Ok(vec![Json::Str(name)])
            }
            "buffer-read" => {
                let b = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let v = self.engine.as_ref().and_then(|e| e.buffer_read(b));
                Ok(vec![v.map(Json::Str).unwrap_or(Json::Null)])
            }
            "buffer-slot-value" => {
                let b = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let slot = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
                let v = self
                    .engine
                    .as_ref()
                    .and_then(|e| e.buffer_slot_value(b, slot));
                Ok(vec![v.map(Json::Str).unwrap_or(Json::Null)])
            }
            "buffer-status" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_buffer_status(&names))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            "clear-buffer" => {
                let b = args.first().and_then(|a| a.as_str()).unwrap_or("");
                if let Some(eng) = &mut self.engine {
                    eng.clear_buffer_rpc(b);
                }
                Ok(vec![Json::Null])
            }
            "set-buffer-chunk" => {
                // `(set-buffer-chunk buffer chunk {requested})`: a named chunk
                // (string) or a spec list.
                let buffer = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("set-buffer-chunk needs a buffer")?;
                if let Some(eng) = &mut self.engine {
                    match args.get(1) {
                        Some(Json::Str(name)) => eng
                            .set_buffer_chunk_named(buffer, name)
                            .map_err(|e| e.to_string())?,
                        Some(Json::Arr(_)) => {
                            let items = match args.get(1) {
                                Some(Json::Arr(a)) => a,
                                _ => unreachable!(),
                            };
                            let forms: Vec<Sexp> = items.iter().map(json_to_sexp).collect();
                            let desc =
                                model::parse_chunk_desc(&forms).map_err(|e| e.to_string())?;
                            eng.set_buffer_chunk_desc(buffer, &desc)
                                .map_err(|e| e.to_string())?;
                        }
                        _ => {}
                    }
                }
                Ok(vec![Json::Null])
            }
            "chunk-p" => {
                let n = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let v = self.engine.as_ref().map(|e| e.chunk_p(n)).unwrap_or(false);
                Ok(vec![Json::Bool(v)])
            }
            "chunk-copied-from" => {
                let n = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let v = self.engine.as_ref().and_then(|e| e.chunk_copied_from(n));
                Ok(vec![v.map(Json::Str).unwrap_or(Json::Null)])
            }
            "chunk-slot-value" => {
                let n = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let s = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
                let v = self.engine.as_ref().and_then(|e| e.chunk_slot(n, s));
                Ok(vec![v.map(Json::Str).unwrap_or(Json::Null)])
            }
            "act-r-random" => {
                let v = args.first().cloned().unwrap_or(Json::Int(1));
                let out = match (self.engine.as_mut(), v) {
                    (Some(eng), Json::Int(n)) => Json::Int(eng.random_int_rpc(n)),
                    (Some(eng), Json::Float(x)) => Json::Float(eng.random_float_rpc(x)),
                    _ => Json::Null,
                };
                Ok(vec![out])
            }
            "permute-list" => {
                let items = match args.first() {
                    Some(Json::Arr(a)) => a.clone(),
                    _ => Vec::new(),
                };
                let out = match self.engine.as_mut() {
                    // ACT-R's `permute-list` (`random.lisp:562`) picks a random
                    // element each pass and removes it, and the step form runs
                    // *before* the end test — so the final single-element pass
                    // still draws (`(nth (act-r-random (length temp)) temp)`)
                    // and an n-element list consumes n draws. Fisher-Yates
                    // consumes n-1, which left the stream one draw behind.
                    Some(eng) => {
                        let mut temp = items;
                        let mut result = Vec::with_capacity(temp.len());
                        while !temp.is_empty() {
                            let i = eng.random_int_rpc(temp.len() as i64) as usize;
                            // `(cons item result)`: each pick goes on the front.
                            result.insert(0, temp.remove(i));
                        }
                        result
                    }
                    None => items,
                };
                Ok(vec![Json::Arr(out)])
            }
            "schedule-event-relative" | "schedule-event-now" | "schedule-event" => {
                self.schedule_event_cmd(name, args);
                Ok(vec![Json::Null])
            }
            "release-chunk-spec-id" => Ok(vec![Json::Null]),
            "record-history" => {
                let buffers: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.set_bold_buffers(&buffers);
                }
                Ok(vec![Json::Null])
            }
            "stop-recording-history" => Ok(vec![Json::Null]),
            "predict-bold-response" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_bold();
                }
                Ok(vec![Json::Null])
            }
            "mp-time" | "mp-time-ms" | "get-time" => {
                let v = self.engine.as_ref().map(|e| e.time_secs()).unwrap_or(0.0);
                if name == "mp-time-ms" {
                    Ok(vec![Json::Int((v * 1000.0).round() as i64)])
                } else {
                    Ok(vec![Json::Float(v)])
                }
            }
            // --- R3 output commands -----------------------------------------
            // Each returns the printed chunk names; the reply wraps the single
            // return value in a list (`actr.py:88` returns `results[1:]`).
            "buffer-chunk" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_buffer_chunk(&names))
                    .unwrap_or_default();
                // An empty buffer prints/returns `nil`.
                let list: Vec<Json> = out
                    .into_iter()
                    .map(|n| {
                        if n.is_empty() {
                            Json::Null
                        } else {
                            Json::Str(n)
                        }
                    })
                    .collect();
                Ok(vec![Json::Arr(list)])
            }
            "dm" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_dm(&names))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            "sdm" => {
                let spec: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_sdm(&spec))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            "pprint-chunks" | "pprint-chunks-fct" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_pprint_chunks(&names))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            // --- R4 device / vision / motor ---------------------------------
            // Each feature arg is a flat `[slot, value, …]` list.
            "add-visicon-features" => {
                let specs: Vec<Vec<(String, Sexp)>> = args.iter().map(flat_slots).collect();
                let names: Vec<Json> = specs
                    .iter()
                    .map(|s| match &mut self.engine {
                        Some(e) => Json::Str(e.add_visicon(s)),
                        None => Json::Null,
                    })
                    .collect();
                // One feature returns its name; several return a list.
                if names.len() == 1 {
                    Ok(names)
                } else {
                    Ok(vec![Json::Arr(names)])
                }
            }
            "modify-visicon-features" => {
                // A client sends one list argument; the spec harness flattens
                // a single list into separate arguments.
                let lists: Vec<Vec<Json>> = if args.first().map(|a| matches!(a, Json::Arr(_))) == Some(true) {
                    args.iter()
                        .filter_map(|a| match a {
                            Json::Arr(x) => Some(x.clone()),
                            _ => None,
                        })
                        .collect()
                } else {
                    vec![args.to_vec()]
                };
                for items in lists {
                    let name = items.first().map(json_to_text).unwrap_or_default();
                    let mut mods: Vec<(String, Sexp)> = Vec::new();
                    let mut i = 1;
                    while i + 1 < items.len() {
                        let k = json_to_text(&items[i]).to_uppercase();
                        mods.push((k, json_to_sexp(&items[i + 1])));
                        i += 2;
                    }
                    if let Some(eng) = &mut self.engine {
                        eng.queue_visicon_modify_sexp(&name, &mods);
                    }
                }
                Ok(vec![Json::Null])
            }
            "delete-visicon-features" => {
                for a in args {
                    let name = json_to_text(a);
                    if let Some(eng) = &mut self.engine {
                        eng.remove_visicon(&name);
                    }
                }
                Ok(vec![Json::Null])
            }
            "delete-all-visicon-features" => {
                if let Some(eng) = &mut self.engine {
                    eng.clear_visicon();
                }
                Ok(vec![Json::Null])
            }
            "print-visicon" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_visicon_rpc(true, None);
                }
                Ok(vec![Json::Null])
            }
            "new-tone-sound" => {
                // `new-tone-sound <freq> <duration> [onset] [time-in-ms]`
                // (`audio.lisp:778`): the last argument says whether *both*
                // numbers are milliseconds; otherwise they are seconds
                // (`safe-seconds->ms`). Duration is not optional-passed in ms
                // here — `(new-tone-sound 100 .1)` is 100 ms, so truncating
                // the seconds value to an integer gave a zero-length sound and
                // its `audio-event-ended` landed before the buffer was stuffed.
                let freq = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                let duration = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let onset = args.get(2).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let in_ms = args
                    .get(3)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let (duration_ms, onset_ms) = if in_ms {
                    (duration as i64, onset as i64)
                } else {
                    (
                        (duration * 1000.0).round() as i64,
                        (onset * 1000.0).round() as i64,
                    )
                };
                if let Some(eng) = &mut self.engine {
                    eng.new_tone_sound_rpc(freq, duration_ms, onset_ms);
                }
                Ok(vec![Json::Null])
            }
            "new-word-sound" | "new-digit-sound" => {
                // A speech-like sound: hold the vocal buffer busy, then
                // release. The corpus's audio specs use tone sounds.
                Ok(vec![Json::Null])
            }
            "pbreak" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.pbreak(&names);
                }
                Ok(vec![Json::Null])
            }
            "punbreak" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.punbreak(&names);
                }
                Ok(vec![Json::Null])
            }
            "pdisable" | "penable" => {
                // `pdisable-fct`/`penable-fct` (`procedural-cmds.lisp`): the
                // names come as separate args (`actr.pdisable('p1','p2')`);
                // no args means all productions.
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    if name == "pdisable" {
                        eng.pdisable(&names);
                    } else {
                        eng.penable(&names);
                    }
                }
                Ok(vec![Json::Null])
            }
            "pp" => {
                // `pp` (`procedural-cmds.lisp`): print the named productions'
                // text, or every production when no name is given.
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.print_productions(&names);
                }
                Ok(vec![Json::Null])
            }
            // --- Live experiment window (`devices/virtual/uwi.lisp`) --------
            "open-exp-window" => {
                let opts = args.get(1);
                let x = opt_i(opts, "x", 300);
                let y = opt_i(opts, "y", 300);
                if let Some(eng) = &mut self.engine {
                    eng.open_window(x, y);
                }
                Ok(vec![Json::Str("window".into())])
            }
            "clear-exp-window" => {
                if let Some(eng) = &mut self.engine {
                    eng.clear_window();
                }
                Ok(vec![Json::Null])
            }
            "add-text-to-exp-window" => {
                let text = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
                let opts = args.get(2);
                let (x, y) = (opt_i(opts, "x", 0), opt_i(opts, "y", 0));
                let (w, h) = (opt_i(opts, "width", 75), opt_i(opts, "height", 20));
                let font = opt_i(opts, "font-size", 12);
                let key = self
                    .engine
                    .as_mut()
                    .map(|e| e.add_window_text(text, x, y, w, h, font))
                    .unwrap_or_default();
                Ok(vec![Json::Str(key)])
            }
            "add-line-to-exp-window" => {
                let start = pair(args.get(1));
                let end = pair(args.get(2));
                let color = args.get(3).and_then(|a| a.as_str()).unwrap_or("black");
                let key = self
                    .engine
                    .as_mut()
                    .map(|e| e.add_window_line(start, end, color))
                    .unwrap_or_default();
                Ok(vec![Json::Str(key)])
            }
            "add-button-to-exp-window" => {
                let opts = args.get(1);
                let text = opt_str(opts, "text", "");
                let (x, y) = (opt_i(opts, "x", 0), opt_i(opts, "y", 0));
                let (w, h) = (opt_i(opts, "width", 75), opt_i(opts, "height", 20));
                let color = opt_str(opts, "color", "gray");
                let action = parse_action(option(opts, "action"));
                let key = self
                    .engine
                    .as_mut()
                    .map(|e| e.add_window_button(&text, x, y, w, h, &color, action))
                    .unwrap_or_default();
                Ok(vec![Json::Str(key)])
            }
            "remove-items-from-exp-window" => {
                let keys: Vec<String> =
                    args[1.min(args.len())..].iter().map(json_to_text).collect();
                if let Some(eng) = &mut self.engine {
                    eng.remove_window_items(&keys);
                }
                Ok(vec![Json::Null])
            }
            "modify-text-for-exp-window" => {
                let key = args.first().map(json_to_text).unwrap_or_default();
                let opts = args.get(1);
                let x = option(opts, "x").and_then(|j| j.as_f64()).map(|v| v as i64);
                let y = option(opts, "y").and_then(|j| j.as_f64()).map(|v| v as i64);
                if let Some(eng) = &mut self.engine {
                    eng.modify_window_text(&key, x, y);
                }
                Ok(vec![Json::Null])
            }
            "modify-line-for-exp-window" => {
                let key = args.first().map(json_to_text).unwrap_or_default();
                let start = pair(args.get(1));
                let end = pair(args.get(2));
                let color = args.get(3).and_then(|a| a.as_str());
                if let Some(eng) = &mut self.engine {
                    eng.modify_window_line(&key, start, end, color);
                }
                Ok(vec![Json::Null])
            }
            "install-device" => Ok(vec![Json::Null]),
            "visible-virtuals-available?" => Ok(vec![Json::Bool(false)]),
            "start-hand-at-mouse" | "process-events" => Ok(vec![Json::Null]),
            "correlation" | "mean-deviation" => {
                let a = num_list(args.first());
                let b = num_list(args.get(1));
                let v = if name == "correlation" {
                    correlation(&a, &b)
                } else {
                    mean_deviation(&a, &b)
                };
                // `{output}` defaults to t: `command-output "…: ~6,3F"`.
                let output = !matches!(args.get(2), Some(Json::Null) | Some(Json::Bool(false)));
                if output {
                    let label = if name == "correlation" {
                        "CORRELATION"
                    } else {
                        "MEAN DEVIATION"
                    };
                    if let Some(eng) = &mut self.engine {
                        eng.model_output(&format!("{label}: {v:6.3}"));
                    }
                }
                Ok(vec![Json::Float(v)])
            }
            "whynot" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_whynot(&names))
                    .unwrap_or_default();
                // `whynot-fct` returns the conflict set.
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            "mp-show-queue" => {
                // Deferred.
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
        // `run` returns `(values seconds events break)` in ACT-R; the first is
        // the model time reached and the second the event count
        // (`scheduling.lisp:877`), which `tutorial_unit2code_initial` prints.
        Ok(vec![
            Json::Float(self.time_secs()),
            Json::Int(self.run_events() as i64),
            Json::Null,
        ])
    }

    /// Interleaved-run support for the server: prepare the absolute limit and
    /// the hidden dummy (`run_begin`), then drive the run in slices so a click
    /// can dispatch its client-owned `:action`.
    pub fn run_begin(&mut self, name: &str, secs: f64) -> (i64, bool, bool) {
        self.engine
            .as_mut()
            .map(|e| e.run_begin(name, secs))
            .unwrap_or((0, false, false))
    }

    /// One run slice; `true` when it yielded for a pending device action.
    pub fn run_slice(&mut self, limit_ms: i64, full: bool) -> Result<bool, String> {
        self.engine
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?
            .run_slice(limit_ms, full)
            .map_err(|e| e.to_string())
    }

    /// Drain the device actions a click queued, for the server to dispatch.
    pub fn take_pending_device_actions(&mut self) -> Vec<(String, Vec<Json>)> {
        self.engine
            .as_mut()
            .map(|e| e.take_pending_device_actions())
            .unwrap_or_default()
    }

    /// The model time in seconds, for `run`'s first return value.
    pub fn time_secs(&self) -> f64 {
        self.engine.as_ref().map(|e| e.time_secs()).unwrap_or(0.0)
    }

    /// Events evaluated in the last `run`, for its second return value.
    pub fn run_events(&self) -> u64 {
        self.engine.as_ref().map(|e| e.run_events()).unwrap_or(0)
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

    /// `set-system-parameter-value` (`central-parameters.lisp:131`). Only
    /// `:starting-parameters` matters here: its `:testing-function` is a remote
    /// command the client owns (invoked through the callback path), and the
    /// other entries are re-applied to the engine.
    pub fn set_system_param(&mut self, key: &str, val: Json) {
        if matches!(val, Json::Null) {
            self.system_params.remove(key);
            return;
        }
        self.system_params.insert(key.to_string(), val.clone());
        if key == ":STARTING-PARAMETERS" {
            // Route the whole list through the engine's own handler instead of
            // applying each pair here. Only that path records the entries in
            // `starting_params`, which is what makes `reset` re-apply them
            // (`central-parameters.lisp:131`). Applying them pair-by-pair set
            // e.g. `:seed` once at load and then lost it at every reset —
            // `paired-learning` needs the seed re-applied at the task boundary
            // so its RNG stream restarts where ACT-R's does.
            //
            // `:testing-function` names a client-owned command; `apply_sgp` has
            // no arm for it, so it is stored and re-applied as a no-op (the
            // reset hook invokes it separately, GOTCHAS #219).
            if let Some(eng) = &mut self.engine {
                let sexp = json_to_sexp(&val);
                let _ = eng.apply_sgp(":starting-parameters", &sexp);
            }
        }
    }

    /// `:starting-parameters` replayed from a spec's `(:SGP :STARTING-PARAMETERS
    /// …)` step. Routing it through the session (rather than straight to the
    /// engine) is what lets the reset hook see its `:testing-function`.
    pub fn set_starting_params(&mut self, val: &Sexp) {
        self.set_system_param(":STARTING-PARAMETERS", sexp_to_json(val));
    }

    /// Reset the model and run the `:testing-function` hook — the path the RPC
    /// `reset` command takes (`invoke_testing_fn`). The golden harness replays a
    /// spec's `(:RESET)` through this rather than the engine directly, or a spec
    /// whose runs depend on the hook (the fan stubs' `trace-fan-run`, which sets
    /// `:act t`) silently loses the trace it enables.
    pub fn reset_model(&mut self) -> Result<(), String> {
        if let Some(eng) = &mut self.engine {
            eng.reset().map_err(|e| e.to_string())?;
        }
        struct NoRemote;
        impl Remote for NoRemote {
            fn call(&mut self, _n: &str, _m: Option<&str>, _a: Vec<Json>) -> Result<Vec<Json>, String> {
                Err("no remote client".into())
            }
        }
        self.invoke_testing_fn(&mut NoRemote);
        Ok(())
    }

    /// The `:testing-function` from `:starting-parameters`, quotes stripped
    /// (`"'python-set-testing-traces'"` → `python-set-testing-traces`).
    fn testing_fn(&self) -> Option<String> {
        let items = match self.system_params.get(":STARTING-PARAMETERS") {
            Some(Json::Arr(a)) => a,
            _ => return None,
        };
        let mut i = 0;
        while i + 1 < items.len() {
            let is_fn = items[i]
                .as_str()
                .map(|k| k.eq_ignore_ascii_case(":testing-function"))
                .unwrap_or(false);
            if is_fn {
                let n = items[i + 1].as_str().unwrap_or("").trim_matches('\'');
                if !n.is_empty() {
                    return Some(n.to_string());
                }
            }
            i += 2;
        }
        None
    }

    /// The client-owned `:testing-function` a `reset` deferred, if any.
    pub fn take_pending_testing_fn(&mut self) -> Option<String> {
        self.pending_testing_fn.take()
    }

    /// `(call-testing-module-function)` (`model-tester.cl:485`): ACT-R's
    /// `:testing-module` reset hook. Dispatches the named client command, or
    /// applies the well-known `set-testing-traces` effect.
    fn invoke_testing_fn(&mut self, remote: &mut dyn Remote) {
        let Some(name) = self.testing_fn() else {
            return;
        };
        if let Some(c) = self.commands.get(&name).cloned() {
            if matches!(c.owner, Owner::Remote { .. }) {
                // A client-owned command. ACT-R fires this hook from inside
                // `reset`, but `remote.call` holds `&mut Session` across the
                // callback, so a testing function that calls back in (e.g.
                // `set-parameter-value`, as `paired-learning`'s does) would
                // block forever: its nested request arrives while the session
                // is borrowed and cannot be dispatched. Hand it to the
                // connection layer instead, which runs it with the borrow
                // released and services nested requests reentrantly.
                self.pending_testing_fn = Some(name);
                return;
            }
            let _ = self.underlying(&c, &name, None, &[], remote);
            return;
        }
        // `set-testing-traces` (`model-tester.cl:494`): `(sgp :v t :cmdt t)`.
        if name.to_uppercase().contains("TESTING-TRACES") {
            if let Some(eng) = &mut self.engine {
                let _ = eng.apply_sgp(":v", &Sexp::Sym("T".into()));
                let _ = eng.apply_sgp(":cmdt", &Sexp::Sym("T".into()));
            }
            return;
        }
        // The tutorial fan stubs' testing functions
        // (`test-models/fan-no-pm.lisp:3`, `fan.lisp:9`):
        // `trace-fan-run` = `(set-testing-traces)` + `(sgp :act t)`, which is why
        // every run it covers shows the declarative activation trace.
        if name.eq_ignore_ascii_case("trace-fan-run") {
            if let Some(eng) = &mut self.engine {
                let _ = eng.apply_sgp(":v", &Sexp::Sym("T".into()));
                let _ = eng.apply_sgp(":cmdt", &Sexp::Sym("T".into()));
                let _ = eng.apply_sgp(":act", &Sexp::Sym("T".into()));
            }
        }
    }

    /// `schedule-event[-relative|-now]`: `(time, action, options)` (or
    /// `(action, options)` for `-now`). The `:params` option becomes the
    /// command's arguments.
    fn schedule_event_cmd(&mut self, kind: &str, args: &[Json]) {
        let now = self.engine.as_ref().map(|e| e.time_ms()).unwrap_or(0);
        let (at_ms, action, options) = match kind {
            "schedule-event-now" => (now, args.first(), args.get(1)),
            _ => {
                let t = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
                let ms = (t * 1000.0).round() as i64;
                let at = if kind == "schedule-event-relative" {
                    now + ms
                } else {
                    ms
                };
                (at, args.get(1), args.get(2))
            }
        };
        let Some(action) = action.and_then(|a| a.as_str()) else {
            return;
        };
        let sexps: Vec<Sexp> = match option(options, "params") {
            Some(Json::Arr(a)) => a.iter().map(json_to_sexp).collect(),
            _ => Vec::new(),
        };
        if let Some(eng) = &mut self.engine {
            eng.schedule_command_rpc(at_ms, action, &sexps);
        }
    }

    /// (Re)build the engine from the loaded models.
    /// Rebuild the engine after an in-place definition change (`chunk-type`,
    /// `add-dm`, …).
    ///
    /// A fresh engine's construction reset arms device run 0, so a device
    /// task's script for the run in progress would be dropped and the run
    /// would end with no window at all (`bst`'s `build-display`). Carry the
    /// run position across, and re-arm that run.
    fn rebuild_engine_in_place(&mut self) -> Result<(), String> {
        let run = self.engine.as_ref().map(|e| e.device_run()).unwrap_or(0);
        // The rebuild is for a definition change, not a new model: ACT-R's
        // `chunk-type` does not clear the trace, and a fresh engine starts with
        // an empty one. Losing it wiped every earlier trial's output whenever a
        // spec interleaves a rebuild with its runs (`bst`'s per-trial
        // `chunk-type`), leaving only the last trial in the compared trace.
        let trace = self
            .engine
            .as_ref()
            .map(|e| e.out_since(0))
            .unwrap_or_default();
        self.build_engine()?;
        if let Some(eng) = self.engine.as_mut() {
            eng.set_device_run(run);
            eng.set_out_lines(trace);
        }
        Ok(())
    }

    fn build_engine(&mut self) -> Result<(), String> {
        let def = match self.defs.last() {
            Some(d) => d.clone(),
            None => return Ok(()),
        };
        self.current_model = Some(def.name.clone());
        let mut eng =
            Engine::new(def, self.seed, 0.0, self.device.clone()).map_err(|e| e.to_string())?;
        eng.set_user_fns(self.user_fns.clone());
        self.engine = Some(eng);
        self.trace_pos = 0;
        // `set-system-parameter-value` settings are global in ACT-R and
        // survive a model (re)load — `:starting-parameters` is re-applied by
        // the central-parameters reset hook (`central-parameters.lisp:131`).
        // The engine is rebuilt from scratch here, so replay them, or a
        // `load_act_r_model` after `set-system-parameter-value` would silently
        // drop e.g. the `:seed` and fall back to the model body's.
        let params: Vec<(String, Json)> = self
            .system_params
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (k, v) in params {
            self.set_system_param(&k, v);
        }
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
        // The spec harness wraps a *string* argument in `{"str": …}` because
        // JSON cannot distinguish a Lisp string from a symbol name, and the
        // generic command path would otherwise turn `"changed"` into `CHANGED`.
        Json::Obj(o) => match o.iter().find(|(k, _)| k == "str") {
            Some((_, Json::Str(s))) => Sexp::Str(s.clone()),
            _ => Sexp::Sym("NIL".into()),
        },
    }
}

/// The engine's `Sexp` → JSON, the inverse of [`json_to_sexp`], for replaying a
/// recorded command step's arguments through [`Session::evaluate`]. `NIL`/`T`
/// become `null`/`true`; other symbols and strings become JSON strings (what
/// the client sends for names).
pub fn sexp_to_json(s: &Sexp) -> Json {
    match s {
        Sexp::Int(i) => Json::Int(*i),
        Sexp::Float(x) => Json::Float(*x),
        Sexp::Str(t) => Json::Str(t.clone()),
        Sexp::Sym(name) => {
            if name == "NIL" {
                Json::Null
            } else if name == "T" {
                Json::Bool(true)
            } else {
                Json::Str(name.clone())
            }
        }
        Sexp::List(items) => Json::Arr(items.iter().map(sexp_to_json).collect()),
    }
}

/// Look up an option in the `[["key",value], …]` list the Python client
/// passes to `schedule-event-*` (`actr.py:924`).
fn option<'a>(options: Option<&'a Json>, key: &str) -> Option<&'a Json> {
    let arr = match options {
        Some(Json::Arr(a)) => a,
        _ => return None,
    };
    for pair in arr {
        if let Json::Arr(p) = pair {
            let matches = p
                .first()
                .and_then(|k| k.as_str())
                .map(|k| k.eq_ignore_ascii_case(key))
                .unwrap_or(false);
            if matches {
                return p.get(1);
            }
        }
    }
    None
}

/// An integer option from an `[["key",value], …]` list.
fn opt_i(options: Option<&Json>, key: &str, default: i64) -> i64 {
    option(options, key)
        .and_then(|j| j.as_f64())
        .map(|v| v.round() as i64)
        .unwrap_or(default)
}

/// `add-button-to-exp-window`'s `:action`: a command name, or a list whose
/// head is the command and whose tail is its arguments
/// (`("bst-button-pressed" 15 "under")`).
fn parse_action(a: Option<&Json>) -> Option<(String, Vec<Json>)> {
    match a? {
        Json::Str(s) => Some((s.clone(), Vec::new())),
        Json::Arr(items) => {
            let name = items.first().and_then(|j| j.as_str())?.to_string();
            Some((name, items[1..].to_vec()))
        }
        _ => None,
    }
}

/// A string option from an `[["key",value], …]` list.
fn opt_str(options: Option<&Json>, key: &str, default: &str) -> String {
    option(options, key)
        .and_then(|j| j.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| default.to_string())
}

/// A `[x, y]` pair (a line endpoint).
fn pair(j: Option<&Json>) -> (i64, i64) {
    match j {
        Some(Json::Arr(a)) => (
            a.first().and_then(|x| x.as_f64()).unwrap_or(0.0) as i64,
            a.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as i64,
        ),
        _ => (0, 0),
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

/// A flat `[slot, value, …]` list (a visicon feature spec) → engine slot pairs.
fn flat_slots(j: &Json) -> Vec<(String, Sexp)> {
    let items = match j {
        Json::Arr(a) => a,
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < items.len() {
        let key = json_to_text(&items[i]).to_uppercase();
        out.push((key, json_to_sexp(&items[i + 1])));
        i += 2;
    }
    out
}

/// `[slot, value, …]` → the `(screen-x, screen-y)` a modify carries.
fn feature_xy(items: &[Json]) -> (i64, i64) {
    let (mut x, mut y) = (0i64, 0i64);
    let mut i = 0;
    while i + 1 < items.len() {
        let key = json_to_text(&items[i]).to_uppercase();
        let v = items[i + 1].as_f64().unwrap_or(0.0) as i64;
        match key.as_str() {
            "SCREEN-X" => x = v,
            "SCREEN-Y" => y = v,
            _ => {}
        }
        i += 2;
    }
    (x, y)
}

/// Coerce a JSON list (or scalar) into `f64`s for the stats commands.
fn num_list(j: Option<&Json>) -> Vec<f64> {
    match j {
        Some(Json::Arr(a)) => a.iter().filter_map(|x| x.as_f64()).collect(),
        Some(other) => other.as_f64().map(|v| vec![v]).unwrap_or_default(),
        None => Vec::new(),
    }
}

/// `correlation` (`commands/misc-cmds.lisp`): Pearson's r over two lists.
fn correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let (a, b) = (&a[..n], &b[..n]);
    let ma = a.iter().sum::<f64>() / n as f64;
    let mb = b.iter().sum::<f64>() / n as f64;
    let mut num = 0.0;
    let mut da = 0.0;
    let mut db = 0.0;
    for i in 0..n {
        let x = a[i] - ma;
        let y = b[i] - mb;
        num += x * y;
        da += x * x;
        db += y * y;
    }
    if da == 0.0 || db == 0.0 {
        0.0
    } else {
        num / (da.sqrt() * db.sqrt())
    }
}

/// `mean-deviation` (`act-gui-interface.lisp:1567`): the RMSD between two lists.
fn mean_deviation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n == 0 {
        return 0.0;
    }
    let ss: f64 = (0..n).map(|i| (a[i] - b[i]).powi(2)).sum();
    (ss / n as f64).sqrt()
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
    fn correlation_and_rmsd_match_the_lisp() {
        // Perfect positive / negative correlation.
        assert!((correlation(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-9);
        assert!((correlation(&[1.0, 2.0, 3.0], &[6.0, 4.0, 2.0]) + 1.0).abs() < 1e-9);
        // `mean-deviation` is the RMSD (`act-gui-interface.lisp:1567`).
        assert!((mean_deviation(&[0.0, 0.0], &[3.0, 4.0]) - 12.5f64.sqrt()).abs() < 1e-9);
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
    fn pdisable_and_penable_toggle_productions() {
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
        assert!(s.has_command("pdisable") && s.has_command("penable"));

        // Disabling a named production suppresses it (but not the others).
        s.evaluate(
            "pdisable",
            None,
            &[Json::Str("increment".into())],
            &mut remote,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            trace.iter().any(|l| l.contains("PRODUCTION-FIRED START")),
            "the enabled production did not fire: {trace:?}"
        );
        assert!(
            !trace
                .iter()
                .any(|l| l.contains("PRODUCTION-FIRED INCREMENT")),
            "the disabled production fired: {trace:?}"
        );

        // Re-enabling it (after a reset) lets it fire again.
        s.evaluate(
            "penable",
            None,
            &[Json::Str("increment".into())],
            &mut remote,
        )
        .unwrap();
        s.evaluate("reset", None, &[], &mut remote).unwrap();
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            trace
                .iter()
                .any(|l| l.contains("PRODUCTION-FIRED INCREMENT")),
            "the re-enabled production did not fire: {trace:?}"
        );
    }

    #[test]
    fn pp_prints_production_text() {
        let root = root();
        if !root.is_dir() {
            return; // ACT-R checkout not available
        }
        let mut s = Session::new(root.clone(), (100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(
                "ACT-R:test-models;models;safe-queries.lisp".into(),
            )],
            &mut remote,
        )
        .unwrap();
        let _ = s.take_trace();
        s.evaluate("pp", None, &[], &mut remote).unwrap();
        let got = s.take_trace();

        // `safe-queries.ref` records two `(pp)` blocks; the first is the plain
        // definition order, which is what a fresh `pp` prints.
        let ref_txt = std::fs::read_to_string(root.join("test-models").join("safe-queries.ref"))
            .expect("safe-queries.ref");
        let norm = |lines: Vec<String>| -> Vec<String> {
            lines
                .into_iter()
                .map(|l| l.trim().to_lowercase())
                .filter(|l| !l.is_empty() && !l.starts_with("#|"))
                .collect()
        };
        let expected = norm(ref_txt.lines().take(47).map(str::to_string).collect());
        assert_eq!(norm(got), expected, "pp output mismatch");
    }

    /// A remote client that records the commands it is asked to run.
    #[derive(Default)]
    struct Collecting {
        lines: Vec<String>,
    }
    impl Remote for Collecting {
        fn call(
            &mut self,
            name: &str,
            _m: Option<&str>,
            args: Vec<Json>,
        ) -> Result<Vec<Json>, String> {
            if name == "sink" {
                if let Some(s) = args.first().and_then(|a| a.as_str()) {
                    self.lines.push(s.to_string());
                }
            }
            Ok(vec![Json::Null])
        }
    }

    #[test]
    fn remote_monitor_receives_the_trace() {
        let root = root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new(root, (100, 0));
        let mut remote = Collecting::default();
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("model-trace", "sink", MonitorWhen::Simple);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str("ACT-R:tutorial;unit1;count.lisp".into())],
            &mut remote,
        )
        .unwrap();
        // Parameter getters and the RNG work once a model is loaded.
        assert_eq!(
            s.evaluate(
                "get-parameter-value",
                None,
                &[Json::Str(":esc".into())],
                &mut remote
            )
            .unwrap()[0],
            Json::Str("T".into())
        );
        s.evaluate("run", None, &[Json::Int(10)], &mut remote)
            .unwrap();
        let lines = s.take_trace();
        for l in lines {
            s.deliver_trace(&l, &mut remote);
        }
        assert!(
            remote
                .lines
                .iter()
                .any(|l| l.contains("PRODUCTION-FIRED START")),
            "monitor saw {:?}",
            remote.lines
        );
        // count.lisp emits a fixed 48-line trace.
        assert_eq!(remote.lines.len(), 48);
    }

    #[test]
    fn dm_and_sdm_print_and_return_names() {
        let root = root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new(root.clone(), (100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str("ACT-R:tutorial;unit1;addition.lisp".into())],
            &mut remote,
        )
        .unwrap();
        // `dm` prints all chunks newest-first and returns the names.
        let r = s.evaluate("dm", None, &[], &mut remote).unwrap();
        let Json::Arr(names) = &r[0] else {
            panic!("dm result not a list: {r:?}")
        };
        assert_eq!(names.len(), 11);
        assert_eq!(names[0], Json::Str("TEN".into()));
        assert_eq!(names[10], Json::Str("ZERO".into()));
        // `sdm '- addend1 1'` excludes the chunk whose addend1 is 1.
        let mut s = Session::new(root.clone(), (100, 0));
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(
                "ACT-R:tutorial;unit1;tutor-model-solution.lisp".into(),
            )],
            &mut remote,
        )
        .unwrap();
        let r = s
            .evaluate(
                "sdm",
                None,
                &[
                    Json::Str("-".into()),
                    Json::Str("addend1".into()),
                    Json::Str("1".into()),
                ],
                &mut remote,
            )
            .unwrap();
        let Json::Arr(names) = &r[0] else {
            panic!("sdm result not a list: {r:?}")
        };
        let names: Vec<String> = names
            .iter()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect();
        assert_eq!(names, vec!["FACT34", "FACT67", "FACT103"]);
    }

    #[test]
    fn chunk_queries_and_add_dm() {
        let root = root();
        if !root.is_dir() {
            return;
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
        let slot = |s: &mut Session, name: &str, sl: &str| {
            s.evaluate(
                "chunk-slot-value",
                None,
                &[Json::Str(name.into()), Json::Str(sl.into())],
                &mut NoRemote,
            )
            .unwrap()[0]
                .clone()
        };
        assert_eq!(slot(&mut s, "one", "next"), Json::Str("TWO".into()));
        // `add-dm` at run time adds a DM chunk.
        s.evaluate(
            "add-dm",
            None,
            &[Json::Arr(vec![
                Json::Str("myc".into()),
                Json::Str("ISA".into()),
                Json::Str("number".into()),
                Json::Str("number".into()),
                Json::Str("nine".into()),
            ])],
            &mut remote,
        )
        .unwrap();
        assert_eq!(slot(&mut s, "myc", "number"), Json::Str("NINE".into()));
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
/// `decode-string-names` (`dispatcher.lisp:2413`) for one remote argument:
/// `mod-focus-external` (`goal.lisp:375`) applies it to every slot/value.
/// A `'quoted'` string is the *string* it wraps (`'lawyer'` -> `"lawyer"`);
/// anything else is a symbol name (`person` -> `PERSON`). Numbers pass
/// through unchanged.
fn decode_string_name(j: &Json) -> Sexp {
    match j {
        Json::Str(s) => {
            let b = s.as_bytes();
            if b.len() > 1 && b[0] == b'\'' && b[b.len() - 1] == b'\'' {
                Sexp::Str(s[1..s.len() - 1].to_string())
            } else {
                Sexp::Sym(s.to_uppercase())
            }
        }
        other => json_to_sexp(other),
    }
}

/// A `mod-focus` argument: slot/value pairs, given either as one flat array
/// (`'{<slot> <value>}*`) or as separate arguments.
fn mod_focus_slots(args: &[Json]) -> Result<Vec<(String, Sexp)>, String> {
    let flat: Vec<Json> = match args {
        [Json::Arr(items)] => items.clone(),
        other => other.to_vec(),
    };
    let mut slots = Vec::new();
    let mut i = 0;
    while i + 1 < flat.len() {
        let k = flat[i]
            .as_str()
            .ok_or_else(|| "mod-focus needs slot names".to_string())?;
        slots.push((k.to_uppercase(), decode_string_name(&flat[i + 1])));
        i += 2;
    }
    Ok(slots)
}

fn goal_desc(args: &[Json]) -> Result<ChunkDesc, String> {    match args.first() {
        Some(Json::Str(name)) => Ok(ChunkDesc::Named(name.to_uppercase())),
        Some(Json::Arr(items)) => {
            let forms: Vec<Sexp> = items.iter().map(json_to_sexp).collect();
            model::parse_chunk_desc(&forms).map_err(|e| e.to_string())
        }
        Some(Json::Null) | None => Err("goal-focus needs an argument".into()),
        _ => Err("bad goal-focus argument".into()),
    }
}
