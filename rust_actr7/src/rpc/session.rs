//! RPC session: the command registry plus driving the Rust engine in response
//! to `evaluate <name> <args>` calls from a client.
//!
//! The registry mirrors ACT-R's `dispatcher-command-table`
//! (`framework/dispatcher.lisp:816`): a name maps to a handler plus
//! before/after/simple monitors. Built-in handlers are implemented here; a
//! command the *client* owns (`actr.add_command`) is a `Remote` entry the
//! server invokes by sending an `evaluate` callback back over the wire.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::rc::Rc;

use crate::engine::{
    DeviceSetup, Engine, EngineError, EventHook, EventView, PreResult, ProductionGraphMode,
    StepperSnapshot, TraceKind,
};
use crate::json::Json;
use crate::model::{self, load_models, ChunkDesc, Model, UserDefun};
use crate::sexpr::Sexp;
use crate::trace::Out;

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
    /// The documentation string supplied by `add`.
    pub doc: String,
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
            doc: String::new(),
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
    "step-once",
    "reset",
    "reload",
    "clear-all",
    "load-act-r-model",
    "load-act-r-code",
    "add-act-r-command",
    "remove-act-r-command",
    "start-pipe-connection",
    "goal-focus",
    "mod-focus",
    "set-parameter-value",
    "get-parameter-value",
    "set-system-parameter-value",
    "get-system-parameter-value",
    "current-model",
    "set-current-model",
    "mp-models",
    "creating-model",
    "deleting-model",
    "run-start",
    "run-stop",
    "reset-start",
    "reset-step1",
    "clear-all-start",
    "start-stepper",
    "stop-stepper",
    "step-stepper",
    "update-stepper",
    "set-stepper-ready",
    "set-stepper-tutoring",
    "set-stepper-step-all",
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
    "mod-chunk",
    "mod-buffer-chunk",
    "extend-possible-slots",
    "schedule-event-relative",
    "schedule-event-now",
    "schedule-event",
    "schedule-break",
    "schedule-break-relative",
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
    "print-dm-finsts",
    "print-activation-trace",
    "print-chunk-activation-trace",
    // Environment viewers (`environment/environment-cmds.lisp` + the
    // `printed-*`/`all-productions`/`buffers`/`chunks` families).
    "act-r-version",
    "all-productions",
    "buffers",
    "chunks",
    "model-buffers",
    "multi-buffer-p",
    "get-m-buffer-chunks",
    "used-production-buffers",
    "chunk-filled-slots-list",
    "printed-chunk",
    "dm-slot-filters",
    "filter-dm-chunks",
    "dm-chunk-details",
    "dm-whynot-text",
    "production-details",
    "whynot-text",
    "printed-visicon",
    "printed-audicon",
    "printed-buffer-chunk",
    "printed-buffer-status",
    "production-graph",
    "production-graph-all",
    "production-graph-freq",
    "production-graph-cycles",
    "production-graph-u-cycles",
    "production-graph-runs",
    "production-graph-u-runs",
    "production-graph-utility",
    "sorted-module-names",
    "modules-with-parameters",
    "modules-parameters",
    "printed-parameter-details",
    "whynot",
    "whynot-dm",
    "print-conflict-tree",
    "simulate-retrieval-request",
    "mp-show-queue",
    "mp-queue-text",
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
    "history-data-available",
    "start-incremental-history-data",
    "get-incremental-history-data",
    "save-history-data",
    "predict-bold-response",
    "sgp",
    "spp",
    "sdp",
    "no-output",
    "pp",
    "p",
    "delete-production",
    // Device / vision / motor.
    "add-visicon-features",
    "modify-visicon-features",
    "delete-visicon-features",
    "delete-all-visicon-features",
    "print-visicon",
    "set-visual-center-point",
    "new-tone-sound",
    "new-ongoing-sound",
    "end-ongoing-sound",
    "print-audicon",
    "new-word-sound",
    "new-digit-sound",
    "get-articulation-time",
    "register-articulation-time",
    "get-syllable-count",
    "register-syllable-count",
    "remove-syllable-count",
    "pbreak",
    "punbreak",
    "trigger-reward",
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
    "close-exp-window",
    "select-exp-window",
    "add-virtual-window-handler",
    "remove-virtual-window-handler",
    "visible-virtual-window-mouse-click",
    "install-device",
    "process-events",
    "visible-virtuals-available?",
    "start-hand-at-mouse",
    // The model's own command output: the motor module calls these, and a
    // task monitors them (`monitor-command "output-key" …`). The engine
    // records each press and the server dispatches it to its monitors.
    "output-key",
    "output-speech",
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

/// An inactive model engine owned by the session.
struct NamedEngine {
    name: String,
    engine: Engine,
}

/// A live RPC session: the selected model plus the other loaded models.
pub struct Session {
    model_files: Vec<String>,
    /// The last file that defined a model, for the Environment's Reload.
    last_model_file: Option<String>,
    defs: Vec<Model>,
    /// Source path for each entry of `defs` (None for in-memory builds).
    def_sources: Vec<Option<String>>,
    user_fns: std::collections::HashMap<String, UserDefun>,
    user_vars: std::collections::HashMap<String, Sexp>,
    commands: BTreeMap<String, Command>,
    /// The selected model's live engine.
    engine: Option<Engine>,
    /// All non-selected model engines, in load order.
    inactive: Vec<NamedEngine>,
    /// Model order for `mp-models` and the Environment menu.
    model_order: Vec<String>,
    /// Regression-harness compatibility: each model-bearing file replaces the
    /// previously loaded model set.
    load_replaces_models: bool,
    shared_seq: Rc<Cell<u64>>,
    global_trace: Vec<String>,
    global_trace_kinds: Vec<TraceKind>,
    global_trace_levels: Vec<Out>,
    global_trace_pos: usize,
    global_time_ms: i64,
    global_run_events: u64,
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
    /// Signals (`reset-start`, `reset-step1`, ...) generated by a command and
    /// delivered by the server once the session borrow is released.
    pending_signals: Vec<(String, Option<String>, Vec<Json>)>,
    /// The Environment stepper session, if one is open.
    stepper: Option<Stepper>,
    /// The scripted device the engine is built with (empty by default).
    device: DeviceSetup,
    /// Models that installed a speech/microphone device. A runtime engine
    /// rebuild must restore this flag, as `define-chunks` does not uninstall
    /// the device in ACT-R.
    speech_models: std::collections::HashSet<String>,
    /// Client commands registered to display visible-virtual windows
    /// (`add-virtual-window-handler`, `tools/visible-virtual.lisp:469`).
    vwin_handlers: Vec<String>,
    /// Experiment windows currently displayed, keyed by model name. A model
    /// has at most one live window; the engine owns the window contents.
    windows: HashMap<String, VisWindow>,
    /// Names passed to `record-history` (uppercased); `TRACE-HISTORY` makes
    /// [`Session::take_trace`] also accumulate lines for the history stream.
    hist_recording: std::collections::BTreeSet<String>,
    /// Trace lines recorded while `trace-history` is enabled, with the detail
    /// level and model needed by the Environment's history filter.
    hist_trace: Vec<TraceHistoryRecord>,
    /// In-progress `start-incremental-history-data` jobs.
    hist_jobs: std::collections::HashMap<u64, HistJob>,
    hist_next_id: u64,
}

#[derive(Debug, Clone)]
struct TraceHistoryRecord {
    line: String,
    level: Out,
    model: String,
}

/// An incremental-history transfer: the JSON chunks still to hand out.
struct HistJob {
    chunks: Vec<String>,
    next: usize,
    /// A saved file's metadata comment, JSON-encoded and returned by the
    /// first `get-incremental-history-data` call (the Tcl load path).
    comment: Option<String>,
}

/// A visible-virtual window the Environment is displaying.
#[derive(Debug, Clone)]
struct VisWindow {
    title: String,
    x: i64,
    y: i64,
    // Sent to the handler in the `open` op; kept here for future hit-testing.
    #[allow(dead_code)]
    width: i64,
    #[allow(dead_code)]
    height: i64,
    handlers: Vec<String>,
    items: Vec<String>,
}

/// Mutable state shared between a [`Session`] and its installed event hook.
#[derive(Default)]
struct StepperShared {
    ready: bool,
    step_all: bool,
    tutoring: bool,
    skip_type: Option<String>,
    skip_value: Option<String>,
    skip_time_ms: Option<i64>,
    /// The paused event is allowed to execute (the user pressed Step).
    approved: bool,
    /// The approved event is the next one the engine executes.
    stepped_pending: bool,
    paused: Option<EventView>,
    paused_model: Option<String>,
    pause_reported: bool,
    pause_desc: Option<String>,
    pending_step_desc: Option<String>,
    stepped: Option<EventView>,
    stepped_model: Option<String>,
    last_step_desc: Option<String>,
    update_ready: bool,
    yielded: bool,
    abort_run: bool,
}

impl StepperShared {
    fn should_pause(&self, ev: &EventView) -> bool {
        if self.step_all {
            return true;
        }
        if let Some(kind) = &self.skip_type {
            if !(ev.shown && ev.trace_on) {
                return false;
            }
            return match kind.as_str() {
                "production" => {
                    (ev.action == "production-selected" || ev.action == "production-fired")
                        && self.skip_value.as_deref().is_some_and(|v| {
                            ev.production
                                .as_deref()
                                .is_some_and(|p| p.eq_ignore_ascii_case(v))
                        })
                }
                "time" => self.skip_time_ms.is_some_and(|t| ev.time_ms >= t),
                "module" => self
                    .skip_value
                    .as_deref()
                    .is_some_and(|v| ev.module.eq_ignore_ascii_case(v)),
                _ => false,
            };
        }
        (ev.shown && ev.trace_on)
            || (self.tutoring
                && matches!(
                    ev.action.as_str(),
                    "production-selected" | "conflict-resolution" | "start-retrieval"
                ))
    }
}

struct StepperHook {
    shared: Rc<RefCell<StepperShared>>,
    model: String,
}

impl EventHook for StepperHook {
    fn pre(&mut self, ev: &EventView) -> PreResult {
        let mut s = self.shared.borrow_mut();
        if s.approved {
            s.approved = false;
            s.stepped_pending = true;
            return PreResult::CONTINUE;
        }
        if s.should_pause(ev) {
            s.paused = Some(ev.clone());
            s.paused_model = Some(self.model.clone());
            s.pause_reported = false;
            return PreResult::BREAK;
        }
        PreResult::CONTINUE
    }

    fn post(&mut self, _line: &str, ev: &EventView) {
        let mut s = self.shared.borrow_mut();
        if s.stepped_pending {
            s.stepped_pending = false;
            s.stepped = Some(ev.clone());
            s.stepped_model = Some(self.model.clone());
            s.last_step_desc = s.pending_step_desc.take();
            s.update_ready = true;
            s.yielded = true;
        }
    }

    fn take_yield(&mut self) -> bool {
        let mut s = self.shared.borrow_mut();
        let y = s.yielded;
        s.yielded = false;
        y
    }
}

struct Stepper {
    id: String,
    next_cmd: String,
    update_cmd: String,
    shared: Rc<RefCell<StepperShared>>,
    snapshot: Option<StepperSnapshot>,
    last_action: String,
}

/// Why one [`Session::run_slice_reason`] call returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStop {
    Finished,
    /// The slice hit its event budget; the run is still in progress.
    Continue,
    DeviceAction,
    StepperPause,
    StepperUpdate,
}

impl Session {
    pub fn new(seed: (u64, u64)) -> Session {
        let mut commands = BTreeMap::new();
        for name in BUILTINS {
            commands.insert((*name).to_string(), Command::builtin());
        }
        Session {
            model_files: Vec::new(),
            last_model_file: None,
            defs: Vec::new(),
            def_sources: Vec::new(),
            user_fns: Default::default(),
            user_vars: Default::default(),
            commands,
            engine: None,
            inactive: Vec::new(),
            model_order: Vec::new(),
            load_replaces_models: false,
            shared_seq: Rc::new(Cell::new(0)),
            global_trace: Vec::new(),
            global_trace_kinds: Vec::new(),
            global_trace_levels: Vec::new(),
            global_trace_pos: 0,
            global_time_ms: 0,
            global_run_events: 0,
            current_model: None,
            trace_pos: 0,
            seed,
            symbol_count: 0,
            system_params: Default::default(),
            pending_testing_fn: None,
            pending_signals: Vec::new(),
            stepper: None,
            device: DeviceSetup::default(),
            speech_models: Default::default(),
            vwin_handlers: Vec::new(),
            windows: HashMap::new(),
            hist_recording: Default::default(),
            hist_trace: Vec::new(),
            hist_jobs: Default::default(),
            hist_next_id: 0,
        }
    }

    /// Kept for the regression harness call sites; multi-model is now allowed.
    pub fn set_multi_model_allowed(&mut self, _allowed: bool) {}

    pub fn set_load_replaces_models(&mut self, replace: bool) {
        self.load_replaces_models = replace;
    }

    fn prepare_model_load(&mut self) {
        if !self.load_replaces_models || self.model_order.is_empty() {
            return;
        }
        for name in self.model_order.clone() {
            self.remove_model(&name);
        }
        self.rebuild_global_trace();
    }

    fn engine_for(&self, name: &str) -> Option<&Engine> {
        if self.current_model.as_deref() == Some(name) {
            self.engine.as_ref()
        } else {
            self.inactive
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(name))
                .map(|m| &m.engine)
        }
    }

    fn engine_mut_for(&mut self, name: &str) -> Option<&mut Engine> {
        if self
            .current_model
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
        {
            self.engine.as_mut()
        } else {
            self.inactive
                .iter_mut()
                .find(|m| m.name.eq_ignore_ascii_case(name))
                .map(|m| &mut m.engine)
        }
    }

    fn current_def_mut(&mut self) -> Option<&mut Model> {
        let name = self.current_model.clone()?;
        self.defs
            .iter_mut()
            .find(|d| d.name.eq_ignore_ascii_case(&name))
    }

    fn make_engine(&self, def: &Model) -> Result<Engine, String> {
        let mut eng = Engine::new(def.clone(), self.seed, 0.0, self.device.clone())
            .map_err(|e| e.to_string())?;
        let multi_model = self.model_order.len() > 1
            || self
                .model_order
                .iter()
                .any(|name| !name.eq_ignore_ascii_case(&def.name));
        eng.set_multi_model(multi_model);
        if self
            .speech_models
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&def.name))
        {
            eng.set_speech_device(true);
        }
        eng.set_user_fns(self.user_fns.clone());
        eng.set_user_vars(self.user_vars.clone());
        eng.use_shared_sequence(self.shared_seq.clone());
        eng.apply_model_commands().map_err(|e| e.to_string())?;
        if self.has_any_trace_monitor() {
            eng.replay_model_outputs();
            eng.warn_invalid_goals();
            eng.warn_invalid_hooks();
            eng.warn_invalid_scheduled_commands();
            eng.warn_missing_model_commands();
            eng.replay_reset_warnings();
        }
        if let Some(st) = &self.stepper {
            eng.set_event_hook(Some(Box::new(StepperHook {
                shared: st.shared.clone(),
                model: def.name.clone(),
            })));
        }
        Ok(eng)
    }

    fn sync_multi_model_flags(&mut self) {
        let multi = self.model_order.len() > 1;
        if let Some(engine) = self.engine.as_mut() {
            engine.set_multi_model(multi);
        }
        for model in &mut self.inactive {
            model.engine.set_multi_model(multi);
        }
    }

    fn install_stepper_hook(&mut self, name: &str, eng: &mut Engine) {
        if let Some(st) = &self.stepper {
            eng.set_event_hook(Some(Box::new(StepperHook {
                shared: st.shared.clone(),
                model: name.to_string(),
            })));
        }
    }

    /// Add one model definition and make it the current model. Existing models
    /// are retained, which is the multi-model load behavior.
    fn add_model_definition(&mut self, def: Model, source: Option<String>) -> Result<(), String> {
        let name = def.name.clone();
        if self
            .model_order
            .iter()
            .any(|n| n.eq_ignore_ascii_case(&name))
        {
            self.remove_model(&name);
        }
        let mut eng = self.make_engine(&def)?;
        self.install_stepper_hook(&name, &mut eng);
        self.defs.push(def);
        self.def_sources.push(source);
        self.model_order.push(name.clone());
        if let Some(old) = self.engine.take() {
            if let Some(old_name) = self.current_model.take() {
                self.inactive.push(NamedEngine {
                    name: old_name,
                    engine: old,
                });
            }
        }
        self.engine = Some(eng);
        self.current_model = Some(name);
        self.sync_multi_model_flags();
        self.rebuild_global_trace();
        Ok(())
    }

    fn activate_model(&mut self, name: &str) -> Result<(), String> {
        if self.current_model.as_deref() == Some(name) {
            return Ok(());
        }
        let Some(pos) = self
            .inactive
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case(name))
        else {
            return Err(format!(
                "Model {name:?} is not loaded; multiple models are supported, but this name is unknown."
            ));
        };
        let target = self.inactive.remove(pos);
        let old = self.engine.take();
        let old_name = self.current_model.take();
        if let (Some(old), Some(old_name)) = (old, old_name) {
            self.inactive.push(NamedEngine {
                name: old_name,
                engine: old,
            });
        }
        self.current_model = Some(target.name);
        self.engine = Some(target.engine);
        Ok(())
    }

    fn rebuild_global_trace(&mut self) {
        self.global_trace.clear();
        self.global_trace_kinds.clear();
        self.global_trace_levels.clear();
        if self.model_order.len() <= 1 {
            self.global_trace_pos = 0;
            return;
        }
        let model_width =
            crate::engine::multi::model_column_width(self.model_order.iter().map(String::len));
        for name in &self.model_order {
            if let Some(eng) = self.engine_for(name) {
                let lines = eng.out_since(0);
                let kinds = eng.out_kinds_since(0);
                let levels = eng.out_levels_since(0);
                for (i, line) in lines.into_iter().enumerate() {
                    self.global_trace
                        .push(crate::engine::multi::with_model_column(
                            &line,
                            name,
                            model_width,
                        ));
                    self.global_trace_kinds
                        .push(kinds.get(i).copied().unwrap_or(TraceKind::Model));
                    self.global_trace_levels
                        .push(levels.get(i).copied().unwrap_or(Out::Low));
                }
            }
        }
        self.global_trace_pos = 0;
    }

    fn remove_model(&mut self, name: &str) {
        if let Some(idx) = self
            .defs
            .iter()
            .position(|d| d.name.eq_ignore_ascii_case(name))
        {
            self.defs.remove(idx);
            if idx < self.def_sources.len() {
                self.def_sources.remove(idx);
            }
        }
        self.model_order.retain(|n| !n.eq_ignore_ascii_case(name));
        if self
            .current_model
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
        {
            self.engine = None;
            self.current_model = None;
        }
        self.inactive.retain(|m| !m.name.eq_ignore_ascii_case(name));
        self.speech_models
            .retain(|model| !model.eq_ignore_ascii_case(name));
        self.windows
            .retain(|window_model, _| !window_model.eq_ignore_ascii_case(name));
        self.sync_multi_model_flags();
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
            doc: String::new(),
            connection,
        };
        self.commands.insert(name.to_string(), c);
    }

    pub fn set_command_doc(&mut self, name: &str, doc: &str) {
        if let Some(c) = self.commands.get_mut(name) {
            c.doc = doc.to_string();
        }
    }

    /// The documentation string for a registered command (`check` result 2).
    pub fn command_doc(&self, name: &str) -> Option<String> {
        self.commands.get(name).map(|c| c.doc.clone())
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
        let mut out = self
            .engine
            .as_mut()
            .map(|e| e.take_fired_commands())
            .unwrap_or_default();
        for model in &mut self.inactive {
            out.extend(model.engine.take_fired_commands());
        }
        out
    }

    /// Task commands fired by `!eval!` since the last call, with their
    /// evaluated arguments rendered as Lisp text.
    pub fn take_fired_task_commands(&mut self) -> Vec<(String, Vec<String>)> {
        let mut out = self
            .engine
            .as_mut()
            .map(|e| e.take_fired_task_commands())
            .unwrap_or_default();
        for model in &mut self.inactive {
            out.extend(model.engine.take_fired_task_commands());
        }
        out
    }

    /// A pending embedder `imaginal-action` request, with its command,
    /// rendered arguments, and whether it is a simple action.
    pub fn take_pending_host_action(&mut self) -> Option<(String, Vec<String>, bool)> {
        self.engine
            .as_mut()
            .and_then(|e| e.take_pending_host_action())
    }

    /// Complete a pending embedder `imaginal-action`.
    pub fn complete_host_action(
        &mut self,
        result: Option<&str>,
        simple: bool,
    ) -> Result<(), String> {
        self.engine
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?
            .complete_host_action(result, simple)
            .map_err(|e| e.to_string())
    }

    /// Take the trace lines and their streams since the last call.
    pub fn take_trace_events(&mut self) -> Vec<(String, TraceKind)> {
        let fallback_model = self.current_model.clone().unwrap_or_default();
        let collected: Vec<(String, TraceKind, Out, String)> = if self.model_order.len() > 1 {
            let pos = self.global_trace_pos;
            let lines = self.global_trace[pos.min(self.global_trace.len())..].to_vec();
            let kinds = self.global_trace_kinds[pos.min(self.global_trace_kinds.len())..].to_vec();
            let levels =
                self.global_trace_levels[pos.min(self.global_trace_levels.len())..].to_vec();
            self.global_trace_pos = self.global_trace.len();
            lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| {
                    let model =
                        trace_model_from_line(&line).unwrap_or_else(|| fallback_model.clone());
                    (
                        line,
                        kinds.get(i).copied().unwrap_or(TraceKind::Model),
                        levels.get(i).copied().unwrap_or(Out::Low),
                        model,
                    )
                })
                .collect()
        } else if let Some(eng) = &self.engine {
            let pos = self.trace_pos;
            let lines = eng.out_since(pos);
            let kinds = eng.out_kinds_since(pos);
            let levels = eng.out_levels_since(pos);
            self.trace_pos = eng.out_len();
            lines
                .into_iter()
                .enumerate()
                .map(|(i, line)| {
                    (
                        line,
                        kinds.get(i).copied().unwrap_or(TraceKind::Model),
                        levels.get(i).copied().unwrap_or(Out::Low),
                        fallback_model.clone(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        // `record-history "trace-history"` captures the complete stream.
        if self.hist_recording.contains("TRACE-HISTORY") {
            self.hist_trace
                .extend(
                    collected
                        .iter()
                        .map(|(line, _, level, model)| TraceHistoryRecord {
                            line: line.clone(),
                            level: *level,
                            model: model.clone(),
                        }),
                );
        }
        collected
            .into_iter()
            .map(|(line, kind, _, _)| (line, kind))
            .collect()
    }

    /// Take just the trace text since the last call.
    pub fn take_trace(&mut self) -> Vec<String> {
        self.take_trace_events()
            .into_iter()
            .map(|(line, _)| line)
            .collect()
    }

    /// The record JSON strings for a history stream, or `None` if the stream
    /// is not supported.
    fn history_records(&self, name: &str) -> Option<Vec<String>> {
        match name {
            "TRACE-HISTORY" => Some(self.hist_trace.iter().map(trace_record).collect()),
            "RETRIEVAL-HISTORY" => Some(
                self.engine
                    .as_ref()
                    .map(|e| e.retrieval_history().iter().map(retrieval_record).collect())
                    .unwrap_or_default(),
            ),
            "BUFFER-HISTORY" => Some(
                self.engine
                    .as_ref()
                    .map(|e| {
                        e.buffer_history()
                            .into_iter()
                            .map(|(t, rows)| buffer_history_record(t, &rows))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            "VISICON-HISTORY" => Some(
                self.engine
                    .as_ref()
                    .map(|e| {
                        e.visicon_history()
                            .iter()
                            .map(|(t, items)| visicon_history_record(*t, items))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            "AUDICON-HISTORY" => Some(
                self.engine
                    .as_ref()
                    .map(|e| {
                        e.audicon_history()
                            .iter()
                            .map(|(t, items)| visicon_history_record(*t, items))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            "BUFFER-TRACE" => Some(
                self.engine
                    .as_ref()
                    .map(|e| {
                        e.buffer_trace()
                            .into_iter()
                            .map(|(t, rows)| buffer_trace_record(t, &rows))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            // Graphic traces are processors over `buffer-trace`.
            "HORIZONTAL-GRAPHIC-TRACE" | "VERTICAL-GRAPHIC-TRACE" => {
                let vertical = name == "VERTICAL-GRAPHIC-TRACE";
                self.engine.as_ref().map(|e| {
                    let now_ms = (e.time_secs() * 1000.0).round() as i64;
                    graphic_trace_items(&e.buffer_trace(), vertical, now_ms)
                })
            }
            "BLENDING-TRACE" => Some(blending_trace_records(
                self.engine
                    .as_ref()
                    .map(|e| e.blending_history())
                    .unwrap_or(&[]),
            )),
            "BLENDING-ENVIRONMENT-HISTORY" => {
                self.engine.as_ref().map(blending_environment_records)
            }
            "PRODUCTION-HISTORY" => self.engine.as_ref().map(production_history_records),
            // `production-grid` is a processor over `production-history`.
            "PRODUCTION-GRID" => self.engine.as_ref().map(production_grid_records),
            // BOLD predictions are processors over `buffer-trace`.
            "BOLD-PREDICTION-WITH-TIME" | "BOLD-PREDICTION-WITH-TIME-SCALED" => {
                let scaled = name == "BOLD-PREDICTION-WITH-TIME-SCALED";
                self.engine
                    .as_ref()
                    .map(|e| e.bold_prediction_with_times(scaled))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|j| j.write())
                    .collect::<Vec<_>>()
                    .into()
            }
            _ => None,
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

    /// Model names in load order (`mp-models`).
    pub fn models(&self) -> Vec<String> {
        self.model_order.clone()
    }

    /// The live engine, for a harness that drives non-command steps at the
    /// engine level while routing commands through [`Session::evaluate`].
    pub fn engine_mut(&mut self) -> Option<&mut Engine> {
        self.engine.as_mut()
    }

    /// Whether more than one model is loaded.
    pub fn is_multi(&self) -> bool {
        self.model_order.len() > 1
    }

    /// Drive all loaded models on the shared meta-process clock.
    pub fn run_multi_secs(&mut self, secs: f64, _trace: bool, width: usize) -> Result<(), String> {
        if !self.is_multi() {
            return self
                .engine
                .as_mut()
                .ok_or_else(|| "no model loaded".to_string())?
                .run_secs(secs)
                .map_err(|e| e.to_string());
        }
        let mut trace_lines = std::mem::take(&mut self.global_trace);
        let mut kinds = std::mem::take(&mut self.global_trace_kinds);
        let mut levels = std::mem::take(&mut self.global_trace_levels);
        let mut time = self.global_time_ms;
        let events = {
            let mut refs: Vec<(&str, &mut Engine)> = Vec::new();
            if let (Some(name), Some(eng)) = (self.current_model.as_deref(), self.engine.as_mut()) {
                refs.push((name, eng));
            }
            for m in &mut self.inactive {
                refs.push((m.name.as_str(), &mut m.engine));
            }
            crate::engine::multi::run_models(
                &mut refs,
                secs,
                width,
                &mut time,
                &mut trace_lines,
                &mut kinds,
                &mut levels,
            )
            .map_err(|e| e.to_string())?
        };
        self.global_trace = trace_lines;
        self.global_trace_kinds = kinds;
        self.global_trace_levels = levels;
        self.global_time_ms = time;
        self.global_run_events += events;
        Ok(())
    }

    /// Install a scripted device and rebuild the engine around it (the spec
    /// harness's `<name>.device.sexp` sidecar).
    pub fn set_device(&mut self, device: DeviceSetup) -> Result<(), String> {
        self.device = device;
        self.build_engine()
    }

    pub fn set_current_model(&mut self, name: Option<String>) {
        if let Some(name) = name {
            let _ = self.activate_model(&name);
        } else {
            self.current_model = None;
        }
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
        let original_model = self.current_model.clone();
        let switched = if let Some(target) = model {
            if self.current_model.as_deref() != Some(target) {
                self.activate_model(target)?;
                true
            } else {
                false
            }
        } else {
            false
        };
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
        if switched && !name.eq_ignore_ascii_case("set-current-model") {
            if let Some(original) = original_model {
                let _ = self.activate_model(&original);
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
            Owner::Builtin => {
                let previous = self.engine.as_ref().map(|e| e.output_kind());
                if let Some(eng) = &mut self.engine {
                    eng.set_output_kind(command_trace_kind(name));
                }
                let result = self.invoke(name, args, remote);
                if let (Some(eng), Some(previous)) = (&mut self.engine, previous) {
                    eng.set_output_kind(previous);
                }
                result
            }
            Owner::None => Ok(vec![Json::Bool(true)]),
            Owner::Remote { local_name } => {
                let call = local_name.clone().unwrap_or_else(|| name.to_string());
                remote.call(&call, model, args.to_vec())
            }
        }
    }

    /// Route a tagged trace line to its Environment monitor. If the matching
    /// stream has no monitor (older clients often monitor only `model-trace`),
    /// fall back to `model-trace` so that client still sees the output.
    pub fn deliver_trace(&mut self, line: &str, kind: TraceKind, remote: &mut dyn Remote) {
        let desired = match kind {
            TraceKind::Model => "model-trace",
            TraceKind::Command => "command-trace",
            TraceKind::Warning => "warning-trace",
            TraceKind::General => "general-trace",
        };
        let target = if self.has_trace_monitor(desired) {
            desired
        } else {
            "model-trace"
        };
        let Some(c) = self.commands.get(target).cloned() else {
            return;
        };
        if c.before.is_empty() && c.after.is_empty() && c.simple.is_empty() {
            return;
        }
        let mut text = line.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let args = [Json::Str(text)];
        let _ = self.evaluate(target, None, &args, remote);
    }

    fn has_trace_monitor(&self, name: &str) -> bool {
        self.commands
            .get(name)
            .map(|c| !c.before.is_empty() || !c.after.is_empty() || !c.simple.is_empty())
            .unwrap_or(false)
    }

    fn has_any_trace_monitor(&self) -> bool {
        [
            "model-trace",
            "command-trace",
            "warning-trace",
            "general-trace",
        ]
        .iter()
        .any(|name| self.has_trace_monitor(name))
    }

    /// A warning that is not itself part of the trace.
    fn warn(&self, _msg: &str) {}

    /// Run each registered handler's `check` and return those that will
    /// display the window (`check-with-environment-for-visible-virtuals`,
    /// `tools/visible-virtual.lisp:500`).
    fn vwin_check(&mut self, remote: &mut dyn Remote) -> Vec<String> {
        let handlers = self.vwin_handlers.clone();
        let mut display = Vec::new();
        for h in handlers {
            if let Ok(v) = self.evaluate(&h, None, &[Json::Str("check".into())], remote) {
                let yes = v
                    .first()
                    .map(|x| !matches!(x, Json::Bool(false) | Json::Null))
                    .unwrap_or(false);
                if yes {
                    display.push(h);
                }
            }
        }
        display
    }

    /// The visible window owned by `model`, if one is open.
    fn window_for(&self, model: &str) -> Option<&VisWindow> {
        self.windows
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(model))
            .map(|(_, window)| window)
    }

    /// The visible window owned by the selected model.
    fn current_window(&self) -> Option<&VisWindow> {
        self.current_model
            .as_deref()
            .and_then(|name| self.window_for(name))
    }

    fn current_window_mut(&mut self) -> Option<&mut VisWindow> {
        let name = self.current_model.clone()?;
        self.windows
            .iter_mut()
            .find(|(key, _)| key.eq_ignore_ascii_case(&name))
            .map(|(_, window)| window)
    }

    /// Find the model that owns a displayed window title. The selected model
    /// wins when titles collide; otherwise model order makes the result stable.
    fn window_owner_by_title(&self, title: &str) -> Option<String> {
        if self
            .current_model
            .as_deref()
            .and_then(|name| self.window_for(name))
            .is_some_and(|window| window.title == title)
        {
            return self.current_model.clone();
        }
        self.model_order
            .iter()
            .find(|name| {
                self.window_for(name)
                    .is_some_and(|window| window.title == title)
            })
            .cloned()
    }

    /// Send a display op to one model's window handlers.
    fn vwin_send_to(&mut self, model: &str, op: &[Json], remote: &mut dyn Remote) {
        let Some(w) = self.window_for(model).cloned() else {
            return;
        };
        for h in w.handlers {
            let _ = self.evaluate(&h, Some(model), op, remote);
        }
    }

    /// The selected model's visible window title (or empty).
    fn vwin_title(&self) -> String {
        self.current_window()
            .map(|w| w.title.clone())
            .unwrap_or_default()
    }

    /// Close and forget one model's visible window.
    fn close_model_window(&mut self, model: &str, remote: &mut dyn Remote) {
        let Some(window) = self.window_for(model).cloned() else {
            return;
        };
        self.vwin_send_to(
            model,
            &[Json::Str("close".into()), Json::Str(window.title)],
            remote,
        );
        self.windows
            .retain(|name, _| !name.eq_ignore_ascii_case(model));
    }

    /// Close every visible experiment window (clear-all / reload).
    fn close_all_windows(&mut self, remote: &mut dyn Remote) {
        let models: Vec<String> = self.windows.keys().cloned().collect();
        for model in models {
            self.close_model_window(&model, remote);
        }
    }

    /// The underlying handler for a built-in command.
    fn invoke(
        &mut self,
        name: &str,
        args: &[Json],
        remote: &mut dyn Remote,
    ) -> Result<Vec<Json>, String> {
        match name {
            // The Rust server has no pipe transport. Returning false makes the
            // tutorial client keep using its socket, just as it does when the
            // Lisp server reports that pipes are unavailable.
            "start-pipe-connection" => Ok(vec![Json::Bool(false)]),
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
                let name = args.first().and_then(|a| a.as_str()).map(str::to_string);
                if let Some(n) = &name {
                    self.activate_model(n)?;
                } else {
                    self.current_model = None;
                }
                Ok(vec![Json::Bool(true)])
            }
            "mp-models" => Ok(vec![Json::Arr(
                self.model_order.iter().cloned().map(Json::Str).collect(),
            )]),
            // `add-act-r-command ... nil` signals (`meta-process.lisp:424,
            // :448`): they have no underlying function, only monitors.
            "creating-model" | "deleting-model" | "run-start" | "run-stop" | "reset-start"
            | "reset-step1" | "clear-all-start" => Ok(vec![Json::Bool(true)]),
            "start-stepper" => self.start_stepper(args),
            "stop-stepper" => self.stop_stepper(args),
            "step-stepper" => self.step_stepper(args),
            "update-stepper" => self.update_stepper(args),
            "set-stepper-ready" => self.set_stepper_ready(args),
            "set-stepper-tutoring" => self.set_stepper_tutoring(args),
            "set-stepper-step-all" => self.set_stepper_step_all(args),
            "load-act-r-model" | "load-act-r-code" => {
                let path = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("load-act-r-model requires a filesystem path")?;
                self.load(path, remote)?;
                Ok(vec![Json::Bool(true)])
            }
            "add-act-r-command" | "remove-act-r-command" => {
                let task_name = args.first().map(json_to_sexp).and_then(|s| match s {
                    Sexp::Sym(n) | Sexp::Str(n) => Some(n),
                    _ => None,
                });
                if let (Some(task_name), Some(eng)) = (task_name, &mut self.engine) {
                    if name == "add-act-r-command" {
                        eng.register_task_command(&task_name);
                    } else {
                        eng.remove_task_command(&task_name);
                    }
                }
                Ok(vec![Json::Bool(true)])
            }
            "reset" => {
                self.queue_signal("reset-start", Vec::new());
                if self.model_order.is_empty() {
                    if let Some(eng) = &mut self.engine {
                        eng.reset().map_err(|e| e.to_string())?;
                    }
                    for model in &mut self.inactive {
                        model.engine.reset().map_err(|e| e.to_string())?;
                    }
                } else {
                    // ACT-R resets models in meta-process order, not with the
                    // selected model first. The order determines the shared
                    // FIFO sequence for the resulting 0.000 events.
                    for name in self.model_order.clone() {
                        if let Some(eng) = self.engine_mut_for(&name) {
                            eng.reset().map_err(|e| e.to_string())?;
                        }
                    }
                }
                if self.is_multi() {
                    self.global_time_ms = 0;
                    self.global_run_events = 0;
                    self.rebuild_global_trace();
                }
                self.queue_signal("reset-step1", Vec::new());
                // ACT-R's `:testing-module` reset hook fires after the model
                // reset (`model-tester.cl:485`).
                self.invoke_testing_fn(remote);
                Ok(vec![Json::Bool(true)])
            }
            "run" | "run-full-time" | "run-until-time" | "run-n-events" | "step-once" => {
                self.run_cmd(name, args)
            }
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
            "model-output" | "command-output" | "act-r-output" => {
                if let Some(engine) = &mut self.engine {
                    let text = args.iter().map(json_to_text).collect::<Vec<_>>().join(" ");
                    engine.model_output(&text);
                }
                Ok(vec![Json::Null])
            }
            "print-warning" => {
                if let Some(engine) = &mut self.engine {
                    let text = args.iter().map(json_to_text).collect::<Vec<_>>().join(" ");
                    engine.emit_warning(&text);
                }
                Ok(vec![Json::Null])
            }
            "get-parameter-value" => {
                let key = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing parameter")?;
                let upper = key.to_uppercase();
                let v = self.engine.as_ref().and_then(|e| match upper.as_str() {
                    ":SEED" => Some(match e.seed() {
                        Some((s, c)) => Json::Arr(vec![Json::Int(s as i64), Json::Int(c as i64)]),
                        None => Json::Null,
                    }),
                    ":V" => Some(if e.get_param(":V").as_deref() == Some("T") {
                        Json::Bool(true)
                    } else {
                        Json::Null
                    }),
                    _ => e.get_param(key).map(Json::Str),
                });
                Ok(vec![v.unwrap_or(Json::Null)])
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
            "clear-all" => {
                self.queue_signal("clear-all-start", Vec::new());
                for name in self.model_order.clone() {
                    self.queue_signal("deleting-model", vec![Json::Str(name)]);
                }
                self.close_all_windows(remote);
                self.defs.clear();
                self.def_sources.clear();
                self.engine = None;
                self.inactive.clear();
                self.model_order.clear();
                self.current_model = None;
                self.model_files.clear();
                self.last_model_file = None;
                self.global_trace.clear();
                self.global_trace_kinds.clear();
                self.global_trace_levels.clear();
                self.global_trace_pos = 0;
                self.global_time_ms = 0;
                self.global_run_events = 0;
                self.trace_pos = 0;
                Ok(vec![Json::Bool(true)])
            }
            "reload" => {
                let path = self
                    .last_model_file
                    .clone()
                    .ok_or_else(|| "No model file recorded to reload.".to_string())?;
                let clears_all = std::fs::read_to_string(&path)
                    .map(|src| model::source_clears_all(&src))
                    .unwrap_or(false);
                if clears_all {
                    // The file begins with `(clear-all)`. ACT-R's `load`
                    // executes it, which also resets the meta-process clock.
                    self.evaluate("clear-all", None, &[], remote)?;
                } else {
                    let replace: Vec<String> = self
                        .def_sources
                        .iter()
                        .enumerate()
                        .filter_map(|(i, src)| {
                            (src.as_deref() == Some(path.as_str()))
                                .then(|| self.defs.get(i).map(|d| d.name.clone()))
                                .flatten()
                        })
                        .collect();
                    for name in &replace {
                        self.queue_signal("deleting-model", vec![Json::Str(name.clone())]);
                        self.close_model_window(name, remote);
                        self.remove_model(name);
                    }
                }
                let created = self.load_file(&path, remote, false)?;
                for name in created {
                    self.queue_signal("creating-model", vec![Json::Str(name)]);
                }
                Ok(vec![Json::Bool(true)])
            }
            // `(sgp :key …)`: a getter when every arg is a keyword, else a
            // setter (`framework/printing.lisp`). `spp`/`sdp` getters are the
            // per-production/sub-symbolic forms; production params are
            // returned as NIL for now.
            "sgp" | "sdp" => {
                // `sdp <chunk…>` prints each chunk's declarative parameter
                // block. Keyword-only `sdp` remains the parameter getter.
                if name == "sdp"
                    && (args.is_empty()
                        || args
                            .iter()
                            .any(|a| !a.as_str().map(|s| s.starts_with(':')).unwrap_or(false)))
                {
                    let names: Vec<String> = args
                        .iter()
                        .filter_map(|a| a.as_str().map(str::to_string))
                        .collect();
                    if let Some(eng) = &mut self.engine {
                        eng.print_sdp_chunks(&names);
                    }
                    return Ok(vec![Json::Null]);
                }
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
                    if name == "sgp" {
                        if let Some(eng) = &mut self.engine {
                            for a in args {
                                if let Some(k) = a.as_str() {
                                    eng.print_sgp_get(k);
                                }
                            }
                        }
                    }
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
                    if i + 1 < args.len() && !args[i + 1].as_str().unwrap_or("").starts_with(':') {
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
                if self.engine.is_none() {
                    return Ok(vec![Json::Null]);
                }
                if keys.is_empty() {
                    // `(spp <name>…)`: print the full parameter block for each
                    // named (or every) production (`production-parameter-fct`,
                    // `utility-and-reward-1.lisp:678`). The RPC client wraps
                    // this in `no-output`, which drops the printed lines.
                    let prods = if names.is_empty() {
                        self.engine.as_ref().unwrap().production_names()
                    } else {
                        names.clone()
                    };
                    let eng = self.engine.as_mut().unwrap();
                    for n in &prods {
                        eng.print_prod_params_by_name(n);
                    }
                    return Ok(vec![Json::Null]);
                }
                let eng = self.engine.as_ref().unwrap();
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
                let mut extended = Vec::new();
                let mut warnings = Vec::new();
                let mut new_defined = Vec::new();
                if let Some(m) = self.current_def_mut() {
                    let defined_before = m.defined_chunks.len();
                    for c in &chunks {
                        let Json::Arr(items) = c else { continue };
                        let forms: Vec<Sexp> = items.iter().map(json_to_sexp).collect();
                        let sexp = Sexp::List(forms);
                        if is_dm {
                            let chunk = model::parse_dm_entry(&sexp).map_err(|e| e.to_string())?;
                            // `chunks.lisp:1233`: a slot outside the type is
                            // extended with a print-warning (not a model
                            // warning), so it appears even when style warnings
                            // are disabled.
                            if let Some(ctype) = chunk.ctype.as_deref() {
                                if let Some(ct) = m
                                    .chunk_types
                                    .iter()
                                    .find(|ct| ct.name.eq_ignore_ascii_case(ctype))
                                {
                                    for (slot, _) in &chunk.slots {
                                        if !ct.slots.iter().any(|s| s.eq_ignore_ascii_case(slot)) {
                                            warnings.push(format!(
                                                "Invalid slot {slot} specified when creating chunk {} with type {}.  Extending chunks with slot named {slot}.",
                                                chunk.name, ct.name
                                            ));
                                            extended.push(slot.clone());
                                        }
                                    }
                                }
                            }
                            m.chunks.push(chunk);
                        } else {
                            model::define_chunk_table_entry(&sexp, m).map_err(|e| e.to_string())?;
                        }
                    }
                    m.apply_chunk_defaults();
                    if !is_dm {
                        new_defined.extend_from_slice(&m.defined_chunks[defined_before..]);
                    }
                }
                if is_dm {
                    self.rebuild_engine_in_place()?;
                } else if let Some(eng) = self.engine.as_mut() {
                    // `define-chunks` only updates the chunk table; rebuilding
                    // would re-arm the reset event and replay trace mid-run.
                    for chunk in &new_defined {
                        eng.add_defined_chunk(chunk).map_err(|e| e.to_string())?;
                    }
                }
                if let Some(eng) = self.engine.as_mut() {
                    for w in warnings {
                        eng.emit_warning(&w);
                    }
                    for s in extended {
                        eng.extend_possible_slots(&s);
                    }
                }
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
                if let Some(m) = self.current_def_mut() {
                    // ACT-R forbids redefinition (it warns); the engine keeps
                    // the latest, as with production redefinition.
                    m.chunk_types
                        .retain(|t| !t.name.eq_ignore_ascii_case(&name));
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
            "mod-buffer-chunk" => {
                let buffer = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("mod-buffer-chunk needs a buffer")?;
                let items: Vec<Json> = match args.get(1) {
                    Some(Json::Arr(a)) => a.clone(),
                    Some(_) => args[1..].to_vec(),
                    None => Vec::new(),
                };
                let mut mods = Vec::new();
                for pair in items.chunks(2) {
                    let name = pair.first().map(json_to_text).unwrap_or_default();
                    let value = pair
                        .get(1)
                        .map(json_to_sexp)
                        .unwrap_or(Sexp::Sym("NIL".into()));
                    mods.push((name.to_uppercase(), value));
                }
                if let Some(eng) = &mut self.engine {
                    eng.mod_buffer_chunk_rpc(buffer, &mods)
                        .map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Null])
            }
            "copy-chunk" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("copy-chunk needs a chunk name")?;
                let copy = self
                    .engine
                    .as_mut()
                    .map(|e| e.copy_chunk_live_rpc(name))
                    .unwrap_or_default();
                Ok(vec![Json::Str(copy)])
            }
            "mod-chunk" => {
                // `(mod-chunk <name> <slot value> ???)`: modify a named chunk
                // in the chunk table or DM.
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("mod-chunk needs a chunk name")?;
                let items: Vec<Json> = match args.get(1) {
                    Some(Json::Arr(a)) => a.clone(),
                    Some(_) => args[1..].to_vec(),
                    None => Vec::new(),
                };
                let mut mods = Vec::new();
                for pair in items.chunks(2) {
                    let slot = pair.first().map(json_to_text).unwrap_or_default();
                    let value = pair
                        .get(1)
                        .map(json_to_sexp)
                        .unwrap_or(Sexp::Sym("NIL".into()));
                    mods.push((slot, value));
                }
                if let Some(eng) = &mut self.engine {
                    eng.mod_chunk_rpc(name, &mods).map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Null])
            }
            "schedule-mod-buffer-chunk" => {
                let buffer = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("schedule-mod-buffer-chunk needs a buffer")?;
                let items: Vec<Json> = match args.get(1) {
                    Some(Json::Arr(a)) => a.clone(),
                    Some(_) => args[1..].to_vec(),
                    None => Vec::new(),
                };
                let mut mods = Vec::new();
                for pair in items.chunks(2) {
                    let name = pair.first().map(json_to_text).unwrap_or_default();
                    let value = pair
                        .get(1)
                        .map(json_to_sexp)
                        .unwrap_or(Sexp::Sym("NIL".into()));
                    mods.push((name.to_uppercase(), value));
                }
                let delay = args.get(2).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let mut delay_ms = (delay * 1000.0).round() as i64;
                let mut priority = 0i64;
                let mut module = "NONE".to_string();
                let mut i = 3;
                while i + 1 < args.len() {
                    let key = args[i].as_str().unwrap_or("").to_uppercase();
                    match key.as_str() {
                        ":PRIORITY" => {
                            priority = args[i + 1].as_f64().unwrap_or(0.0) as i64;
                        }
                        ":MODULE" => {
                            module = json_to_text(&args[i + 1]).to_uppercase();
                        }
                        ":TIME-IN-MS" => {
                            if !matches!(args[i + 1], Json::Bool(false) | Json::Null) {
                                delay_ms = delay as i64;
                            }
                        }
                        _ => {}
                    }
                    i += 2;
                }
                if let Some(eng) = &mut self.engine {
                    eng.schedule_mod_buffer_chunk_rpc(buffer, &mods, delay_ms, &module, priority);
                }
                Ok(vec![Json::Null])
            }
            "extend-possible-slots" => {
                for arg in args {
                    if let Some(slot) = arg.as_str() {
                        if let Some(eng) = &mut self.engine {
                            eng.extend_possible_slots(slot);
                        }
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
                let v = self
                    .engine
                    .as_ref()
                    .and_then(|e| e.chunk_slot_live_value(n, s));
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
            "schedule-break" | "schedule-break-relative" => {
                self.schedule_break_cmd(name == "schedule-break-relative", args);
                Ok(vec![Json::Null])
            }
            "release-chunk-spec-id" => Ok(vec![Json::Null]),
            "record-history" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                for n in &names {
                    let key = n.to_uppercase();
                    if key == "TRACE-HISTORY" && self.hist_recording.insert(key.clone()) {
                        // A fresh recording starts empty.
                        self.hist_trace.clear();
                    } else {
                        self.hist_recording.insert(key);
                    }
                }
                // `record-history` with buffer names drives the BOLD predictor;
                // the history names enable their engine-side recorders.
                if let Some(eng) = &mut self.engine {
                    eng.record_history_names(&names);
                }
                Ok(vec![Json::Null])
            }
            "stop-recording-history" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                for n in &names {
                    self.hist_recording.remove(&n.to_uppercase());
                }
                if let Some(eng) = &mut self.engine {
                    eng.stop_recording_history_names(&names);
                }
                Ok(vec![Json::Null])
            }
            "history-data-available" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_uppercase();
                let detail = (name == "TRACE-HISTORY").then(|| args.get(3)).flatten();
                let any = self
                    .history_records(&name)
                    .map(|r| {
                        !filter_history_records(r, args.get(1), args.get(2), detail).is_empty()
                    })
                    .unwrap_or(false);
                Ok(vec![Json::Bool(any)])
            }
            // Start an incremental transfer of a history stream: returns an id
            // whose chunks are fetched with `get-incremental-history-data`.
            // A non-false third arg names a saved file to load instead of the
            // live data.
            "start-incremental-history-data" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_uppercase();
                let size = args
                    .get(1)
                    .and_then(|a| a.as_f64())
                    .map(|v| v.max(1.0) as usize)
                    .unwrap_or(16000);
                let file = match args.get(2) {
                    Some(Json::Str(s))
                        if !s.is_empty()
                            && !s.eq_ignore_ascii_case("false")
                            && !s.eq_ignore_ascii_case("null") =>
                    {
                        Some(s.clone())
                    }
                    _ => None,
                };
                let (comment, records) = if let Some(f) = file {
                    let text = std::fs::read_to_string(&f)
                        .map_err(|e| format!("start-incremental-history-data: {e}"))?;
                    if text.trim().is_empty() {
                        return Ok(vec![Json::Null]);
                    }
                    match parse_saved_history(&text) {
                        Some((comment, records)) if !records.is_empty() => {
                            (Some(comment.unwrap_or_default()), records)
                        }
                        _ => return Ok(vec![Json::Null]),
                    }
                } else {
                    match self.history_records(&name) {
                        Some(r) if !r.is_empty() => (None, r),
                        _ => return Ok(vec![Json::Null]),
                    }
                };
                let detail = (name == "TRACE-HISTORY").then(|| args.get(5)).flatten();
                let records = filter_history_records(records, args.get(3), args.get(4), detail);
                if records.is_empty() {
                    return Ok(vec![Json::Null]);
                }
                let comment = comment.map(|c| Json::Str(c).write());
                self.hist_next_id += 1;
                let id = self.hist_next_id;
                self.hist_jobs.insert(
                    id,
                    HistJob {
                        chunks: chunk_records(&records, size),
                        next: 0,
                        comment,
                    },
                );
                Ok(vec![Json::Int(id as i64)])
            }
            // The next chunk of an incremental transfer (empty string when
            // done).
            "get-incremental-history-data" => {
                let id = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0) as u64;
                if let Some(job) = self.hist_jobs.get_mut(&id) {
                    if let Some(comment) = job.comment.take() {
                        return Ok(vec![Json::Str(comment)]);
                    }
                    if job.next < job.chunks.len() {
                        let chunk = job.chunks[job.next].clone();
                        job.next += 1;
                        return Ok(vec![Json::Str(chunk)]);
                    }
                    self.hist_jobs.remove(&id);
                }
                Ok(vec![Json::Str(String::new())])
            }
            // Write a history stream to a file.
            "save-history-data" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_uppercase();
                let file = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
                let Some(records) = self.history_records(&name) else {
                    return Ok(vec![Json::Bool(false)]);
                };
                let detail = (name == "TRACE-HISTORY").then(|| args.get(5)).flatten();
                let records = filter_history_records(records, args.get(3), args.get(4), detail);
                if records.is_empty() || file.is_empty() {
                    return Ok(vec![Json::Bool(false)]);
                }
                let data = crate::json::parse(&format!("[{}]", records.join(",")))
                    .unwrap_or_else(|_| Json::Arr(Vec::new()));
                let comment = args
                    .get(2)
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let version = self
                    .engine
                    .as_ref()
                    .map(|e| e.act_r_version())
                    .unwrap_or_default();
                let payload = Json::Obj(vec![
                    ("name".into(), Json::Str(name.clone())),
                    ("date".into(), Json::Str("unknown".into())),
                    ("version".into(), Json::Str(version)),
                    ("comment".into(), Json::Str(comment)),
                    ("data".into(), data),
                ]);
                match std::fs::write(file, payload.write()) {
                    Ok(()) => Ok(vec![Json::Bool(true)]),
                    Err(e) => Err(format!("save-history-data: {e}")),
                }
            }
            "predict-bold-response" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_bold();
                }
                Ok(vec![Json::Null])
            }
            "mp-time" | "mp-time-ms" | "get-time" => {
                // `mp-time` is the meta-process clock. In a multi-model run
                // the current engine can be an inactive model whose own clock
                // has not advanced, so use the shared session clock.
                let v = self.time_secs();
                match name {
                    "mp-time-ms" | "get-time" => Ok(vec![Json::Int((v * 1000.0).round() as i64)]),
                    _ => Ok(vec![Json::Float(v)]),
                }
            }
            // --- R3 output commands -----------------------------------------
            // Each returns the printed chunk names; the reply wraps the single
            // return value in a list (`actr.py:88` returns `results[1:]`).
            "buffer-chunk" => {
                let mut names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                let all = names.is_empty();
                if all {
                    names = self
                        .engine
                        .as_ref()
                        .map(|e| e.buffer_names())
                        .unwrap_or_default();
                    // `model-buffers` does not include the extended-motor
                    // per-hand buffers in the plain `(buffer-chunk)` report.
                    names.retain(|n| !matches!(n.as_str(), "MANUAL-LEFT" | "MANUAL-RIGHT"));
                }
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| {
                        if all {
                            e.print_buffer_headers(&names)
                        } else {
                            e.print_buffer_chunk(&names)
                        }
                    })
                    .unwrap_or_default();
                if all {
                    // `(buffer-chunk)` returns `((buffer chunk) ...)`, omitting
                    // the chunk name when the buffer is empty.
                    let list: Vec<Json> = names
                        .iter()
                        .zip(out)
                        .map(|(buffer, chunk)| {
                            let mut pair = vec![Json::Str(buffer.clone())];
                            if !chunk.is_empty() {
                                pair.push(Json::Str(chunk));
                            }
                            Json::Arr(pair)
                        })
                        .collect();
                    return Ok(vec![Json::Arr(list)]);
                }
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
                let mut names: Vec<String> = Vec::new();
                for a in args {
                    match a {
                        Json::Arr(items) => names
                            .extend(items.iter().filter_map(|x| x.as_str().map(str::to_string))),
                        _ => {
                            if let Some(s) = a.as_str() {
                                names.push(s.to_string());
                            }
                        }
                    }
                }
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_pprint_chunks(&names))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(out.into_iter().map(Json::Str).collect())])
            }
            // --- Environment viewers ---------------------------------------
            "act-r-version" => Ok(vec![Json::Str(
                self.engine
                    .as_ref()
                    .map(|e| e.act_r_version())
                    .unwrap_or_default(),
            )]),
            "all-productions" => Ok(vec![Json::Arr(strings(
                self.engine.as_ref().map(|e| e.all_production_names()),
            ))]),
            "buffers" | "model-buffers" => Ok(vec![Json::Arr(strings(
                self.engine.as_ref().map(|e| e.buffer_names()),
            ))]),
            "chunks" => Ok(vec![Json::Arr(strings(
                self.engine.as_ref().map(|e| e.dm_chunk_names()),
            ))]),
            "multi-buffer-p" => {
                let buffer = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let multi = self
                    .engine
                    .as_ref()
                    .is_some_and(|e| e.is_multi_buffer(buffer));
                Ok(vec![Json::Bool(multi)])
            }
            "get-m-buffer-chunks" => {
                let buffer = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let names = self
                    .engine
                    .as_ref()
                    .map(|e| e.multi_buffer_chunks(buffer))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(strings(Some(names)))])
            }
            "used-production-buffers" => Ok(vec![Json::Arr(strings(
                self.engine.as_ref().map(|e| e.used_production_buffers()),
            ))]),
            "chunk-filled-slots-list" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let slots = self
                    .engine
                    .as_ref()
                    .map(|e| e.chunk_filled_slots(name))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(strings(Some(slots)))])
            }
            "printed-chunk" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self
                    .engine
                    .as_ref()
                    .and_then(|e| e.printed_chunk(name))
                    .unwrap_or_default();
                Ok(vec![Json::Str(text)])
            }
            // The filter dropdown's choices, each a space-joined slot-list
            // string with the `_none_` sentinel first.
            "dm-slot-filters" => Ok(vec![Json::Arr(strings(Some(
                self.engine
                    .as_ref()
                    .map(|e| e.dm_slot_filters())
                    .unwrap_or_else(|| vec!["_none_".to_string()]),
            )))]),
            "filter-dm-chunks" => {
                // The Environment sends the chosen filter as one string
                // ("SLOT1 SLOT2"); a list is accepted too.
                let filter: Vec<String> = match args.first() {
                    Some(Json::Arr(a)) => a
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect(),
                    Some(Json::Str(s)) if !s.eq_ignore_ascii_case("_none_") && s != "null" => {
                        s.split_whitespace().map(str::to_string).collect()
                    }
                    _ => Vec::new(),
                };
                let names = self
                    .engine
                    .as_ref()
                    .map(|e| e.filter_dm_chunks(&filter))
                    .unwrap_or_default();
                Ok(vec![Json::Arr(strings(Some(names)))])
            }
            // `dm-chunk-details` (`environment-cmds.lisp`): the chunk's
            // declarative parameter block followed by its printed form.
            "dm-chunk-details" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self.engine.as_mut().map(|e| {
                    let sdp = e.capture_sdp_text(name);
                    let chunk = e.printed_chunk(name).unwrap_or_default();
                    format!("{sdp}\n{chunk}")
                });
                Ok(vec![Json::Str(text.unwrap_or_default())])
            }
            "dm-whynot-text" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self
                    .engine
                    .as_mut()
                    .map(|e| e.capture_whynot_dm_text(name))
                    .unwrap_or_default();
                Ok(vec![Json::Str(text)])
            }
            // `production-details` (`environment-cmds.lisp`): `spp` followed
            // by the printed production.
            "production-details" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self.engine.as_mut().map(|e| {
                    let spp = e.capture_spp_text(name);
                    let prod = e.printed_production(name).unwrap_or_default();
                    format!("{spp}\n{prod}")
                });
                Ok(vec![Json::Str(text.unwrap_or_default())])
            }
            // "Time: T" then the `whynot` report (suppressed from the trace).
            "whynot-text" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let mark = self.engine.as_ref().map(|e| e.out_len()).unwrap_or(0);
                let time = self.engine.as_ref().map(|e| e.time_secs()).unwrap_or(0.0);
                let names = [Json::Str(name)];
                let _ = self.evaluate("whynot", None, &names, remote);
                let body = self
                    .engine
                    .as_ref()
                    .map(|e| e.out_text_since(mark))
                    .unwrap_or_default();
                if let Some(eng) = self.engine.as_mut() {
                    eng.truncate_out(mark);
                    if self.trace_pos > mark {
                        self.trace_pos = mark;
                    }
                }
                Ok(vec![Json::Str(format!("Time: {time:.3}\n{body}"))])
            }
            "printed-buffer-chunk" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self
                    .engine
                    .as_mut()
                    .map(|e| e.printed_buffer_chunk(name))
                    .unwrap_or_default();
                Ok(vec![Json::Str(text)])
            }
            "printed-buffer-status" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self
                    .engine
                    .as_mut()
                    .map(|e| e.printed_buffer_status(name))
                    .unwrap_or_default();
                Ok(vec![Json::Str(text)])
            }
            "printed-visicon" => {
                let out = self
                    .engine
                    .as_mut()
                    .map(|e| e.print_visicon_rpc(false, None))
                    .unwrap_or_default();
                Ok(vec![Json::Str(out.join("\n"))])
            }
            "printed-audicon" => {
                let out = self
                    .engine
                    .as_ref()
                    .map(|e| e.printed_audicon().join("\n"))
                    .unwrap_or_default();
                Ok(vec![Json::Str(out)])
            }
            "production-graph-all"
            | "production-graph-freq"
            | "production-graph-cycles"
            | "production-graph-u-cycles"
            | "production-graph-runs"
            | "production-graph-u-runs"
            | "production-graph-utility" => {
                let mode = match name {
                    "production-graph-freq" => ProductionGraphMode::Frequencies,
                    "production-graph-cycles" => ProductionGraphMode::Cycles,
                    "production-graph-u-cycles" => ProductionGraphMode::UniqueCycles,
                    "production-graph-runs" => ProductionGraphMode::Runs,
                    "production-graph-u-runs" => ProductionGraphMode::UniqueRuns,
                    "production-graph-utility" => ProductionGraphMode::Utilities,
                    _ => ProductionGraphMode::All,
                };
                let data = self
                    .engine
                    .as_ref()
                    .map(|e| e.production_graph_data(mode).to_json())
                    .unwrap_or(Json::Null);
                Ok(vec![data])
            }
            // The production transition graph (nodes + counted edges) for the
            // Environment's Production Graph view.
            "production-graph" => {
                let (nodes, edges) = self
                    .engine
                    .as_ref()
                    .map(|e| e.production_graph())
                    .unwrap_or_default();
                Ok(vec![Json::Arr(vec![
                    Json::Arr(nodes.into_iter().map(Json::Str).collect()),
                    Json::Arr(
                        edges
                            .into_iter()
                            .map(|(a, b, c)| {
                                Json::Arr(vec![Json::Str(a), Json::Str(b), Json::Int(c as i64)])
                            })
                            .collect(),
                    ),
                ])])
            }
            // The parameter viewer: modules, their parameters, and each
            // parameter's current value (see `Engine::module_parameters`).
            "sorted-module-names" => {
                let mut modules: Vec<String> = self
                    .engine
                    .as_ref()
                    .map(|e| e.module_names().iter().map(|m| m.to_string()).collect())
                    .unwrap_or_default();
                modules.sort();
                Ok(vec![Json::Arr(strings(Some(modules)))])
            }
            "modules-with-parameters" => {
                let mut modules: Vec<String> = self
                    .engine
                    .as_ref()
                    .map(|e| {
                        e.module_parameters()
                            .into_iter()
                            .map(|(m, _)| m.to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                modules.sort();
                Ok(vec![Json::Arr(strings(Some(modules)))])
            }
            "modules-parameters" => {
                let module = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let params: Vec<String> = self
                    .engine
                    .as_ref()
                    .map(|e| {
                        e.module_parameters()
                            .into_iter()
                            .find(|(m, _)| *m == module)
                            .map(|(_, p)| p.into_iter().map(str::to_string).collect())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                Ok(vec![Json::Arr(strings(Some(params)))])
            }
            "printed-parameter-details" => {
                let param = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let text = self
                    .engine
                    .as_mut()
                    .map(|e| e.capture_parameter_details(param))
                    .unwrap_or_default();
                Ok(vec![Json::Str(text)])
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
                let lists: Vec<Vec<Json>> =
                    if args.first().map(|a| matches!(a, Json::Arr(_))) == Some(true) {
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
            "set-visual-center-point" => {
                let x = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
                let y = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                if let Some(eng) = &mut self.engine {
                    eng.set_visual_center_point(x as f32, y as f32);
                }
                Ok(vec![Json::Arr(vec![Json::Float(x), Json::Float(y)])])
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
            "new-ongoing-sound" => {
                // `new-ongoing-sound content delay recode [onset location
                // kind time-in-ms other-features...]` (`audio.lisp:840`).
                let content = match args.first().map(json_to_sexp) {
                    Some(crate::sexpr::Sexp::Str(s)) => s,
                    Some(crate::sexpr::Sexp::Sym(s)) => s,
                    _ => String::new(),
                };
                let delay = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let recode = args.get(2).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let onset = args.get(3).and_then(|a| a.as_f64());
                let location = args
                    .get(4)
                    .map(json_to_text)
                    .unwrap_or_else(|| "EXTERNAL".to_string());
                let kind = args
                    .get(5)
                    .map(json_to_text)
                    .unwrap_or_else(|| "SPEECH".to_string());
                let in_ms = args
                    .get(6)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let (delay_ms, recode_ms, onset_ms) = if in_ms {
                    (delay as i64, recode as i64, onset.map(|v| v as i64))
                } else {
                    (
                        (delay * 1000.0).round() as i64,
                        (recode * 1000.0).round() as i64,
                        onset.map(|v| (v * 1000.0).round() as i64),
                    )
                };
                let other: Vec<crate::sexpr::Sexp> =
                    args.iter().skip(7).map(json_to_sexp).collect();
                let id = self
                    .engine
                    .as_mut()
                    .map(|e| {
                        e.new_ongoing_sound_rpc(
                            content, delay_ms, recode_ms, onset_ms, location, kind, &other,
                        )
                    })
                    .transpose()
                    .map_err(|e| e.to_string())?
                    .unwrap_or(0);
                Ok(vec![Json::Int(id)])
            }
            "end-ongoing-sound" => {
                let id = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                if let Some(eng) = &mut self.engine {
                    eng.end_ongoing_sound_rpc(id).map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Bool(true)])
            }
            "print-audicon" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_audicon_rpc();
                }
                Ok(vec![Json::Null])
            }
            "new-word-sound" => {
                // `new-word-sound <word> [onset] [location] [time-in-ms]`
                // (`audio.lisp:901`): a WORD sound whose duration is the
                // speech module's articulation time.
                let word = args.first().map(json_to_text).unwrap_or_default();
                let onset = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let location = args
                    .get(2)
                    .map(json_to_text)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "EXTERNAL".to_string());
                let in_ms = args
                    .get(3)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let onset_ms = if in_ms {
                    onset as i64
                } else {
                    (onset * 1000.0).round() as i64
                };
                if !word.is_empty() {
                    if let Some(eng) = &mut self.engine {
                        eng.new_word_sound_rpc(word, onset_ms, location);
                    }
                }
                Ok(vec![Json::Null])
            }
            "register-articulation-time" => {
                // `register-articulation-time <string> <time> [time-in-ms]`.
                let word = args.first().map(json_to_text).unwrap_or_default();
                let time = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let in_ms = args
                    .get(2)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let time_ms = if in_ms {
                    time.round() as i64
                } else {
                    (time * 1000.0).round() as i64
                };
                if let Some(eng) = &mut self.engine {
                    eng.register_articulation_time(word.clone(), time_ms);
                }
                Ok(vec![Json::Str(word)])
            }
            "get-articulation-time" => {
                // `get-articulation-time <string> [time-in-ms]`.
                let word = args.first().map(json_to_text).unwrap_or_default();
                let in_ms = args
                    .get(1)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let Some(eng) = &self.engine else {
                    return Ok(vec![Json::Null]);
                };
                let time = eng.get_articulation_time(&word, in_ms);
                Ok(vec![if in_ms {
                    Json::Int(time as i64)
                } else {
                    Json::Float(time)
                }])
            }
            "register-syllable-count" => {
                // `register-syllable-count <word> <syllables>`.
                let word = args.first().map(json_to_text).unwrap_or_default();
                let syllables = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                if word.is_empty() || !(1..=255).contains(&syllables) {
                    return Ok(vec![Json::Null]);
                }
                if let Some(eng) = &self.engine {
                    eng.register_syllable_count(&word, syllables as u8);
                }
                Ok(vec![Json::Str(word)])
            }
            "get-syllable-count" => {
                let word = args.first().map(json_to_text).unwrap_or_default();
                let Some(eng) = &self.engine else {
                    return Ok(vec![Json::Null]);
                };
                Ok(vec![match eng.get_syllable_count(&word) {
                    Some(count) => Json::Int(count as i64),
                    None => Json::Null,
                }])
            }
            "remove-syllable-count" => {
                let word = args.first().map(json_to_text).unwrap_or_default();
                let Some(eng) = &self.engine else {
                    return Ok(vec![Json::Null]);
                };
                Ok(vec![if eng.remove_syllable_count(&word) {
                    Json::Str(word)
                } else {
                    Json::Null
                }])
            }
            "new-digit-sound" => {
                // `new-digit-sound <digit> [onset] [time-in-ms]`.
                let digit = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                let onset = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let in_ms = args
                    .get(2)
                    .map(|a| matches!(a, Json::Bool(true)))
                    .unwrap_or(false);
                let onset_ms = if in_ms {
                    onset as i64
                } else {
                    (onset * 1000.0).round() as i64
                };
                if let Some(eng) = &mut self.engine {
                    eng.new_digit_sound_rpc(digit, onset_ms);
                }
                Ok(vec![Json::Null])
            }
            "trigger-reward" => {
                let value = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0) as f32;
                if let Some(eng) = &mut self.engine {
                    eng.trigger_reward(value);
                }
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
            "delete-production" => {
                let name = args.first().and_then(|a| a.as_str()).unwrap_or_default();
                if let Some(eng) = &mut self.engine {
                    eng.delete_production(name);
                }
                Ok(vec![Json::Null])
            }
            "p" => {
                // Runtime production definition used by test drivers:
                // `(p name ... ==> ...)`. The flat argument list is rejoined
                // into the Lisp form and parsed by the engine.
                fn text(a: &Json) -> String {
                    match a {
                        Json::Null => "NIL".to_string(),
                        Json::Bool(true) => "T".to_string(),
                        Json::Bool(false) => "NIL".to_string(),
                        Json::Int(i) => i.to_string(),
                        Json::Float(x) => format!("{x}"),
                        Json::Str(s) => s.clone(),
                        Json::Arr(items) => {
                            format!("({})", items.iter().map(text).collect::<Vec<_>>().join(" "))
                        }
                        Json::Obj(_) => "NIL".to_string(),
                    }
                }
                let body: Vec<String> = args.iter().map(text).collect();
                if let Some(eng) = &mut self.engine {
                    let text = format!("(p {})", body.join(" "));
                    eng.define_production_text(&text)
                        .map_err(|e| e.to_string())?;
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
                let title = args
                    .first()
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let opts = args.get(1);
                let x = opt_i(opts, "x", 300);
                let y = opt_i(opts, "y", 300);
                let width = opt_i(opts, "width", 300);
                let height = opt_i(opts, "height", 300);
                let visible = opt_bool(opts, "visible", true);
                let model = self.current_model.clone().unwrap_or_default();
                // One live experiment window per model; a second open replaces
                // the first. The engine owns the window contents, while the
                // session tracks display metadata and item ids.
                self.close_model_window(&model, remote);
                if let Some(eng) = &mut self.engine {
                    eng.open_window(x, y);
                }
                // `open-exp-window` builds a visible-virtual window when a
                // client handler accepts it (`check`), else a headless one.
                if visible {
                    let handlers = self.vwin_check(remote);
                    if !handlers.is_empty() {
                        self.windows.insert(
                            model.clone(),
                            VisWindow {
                                title: title.clone(),
                                x,
                                y,
                                width,
                                height,
                                handlers,
                                items: Vec::new(),
                            },
                        );
                        self.vwin_send_to(
                            &model,
                            &[
                                Json::Str("open".into()),
                                Json::Str(title),
                                Json::Int(x),
                                Json::Int(y),
                                Json::Int(width),
                                Json::Int(height),
                            ],
                            remote,
                        );
                    }
                }
                Ok(vec![Json::Str("window".into())])
            }
            "close-exp-window" => {
                if let Some(model) = self.current_model.clone() {
                    self.close_model_window(&model, remote);
                }
                Ok(vec![Json::Bool(true)])
            }
            "select-exp-window" => {
                let title = self.vwin_title();
                if let Some(model) = self.current_model.clone() {
                    if !title.is_empty() {
                        self.vwin_send_to(
                            &model,
                            &[Json::Str("select".into()), Json::Str(title)],
                            remote,
                        );
                    }
                }
                Ok(vec![Json::Bool(true)])
            }
            "add-virtual-window-handler" => {
                if let Some(h) = args.first().and_then(|a| a.as_str()) {
                    if !self.vwin_handlers.iter().any(|x| x == h) {
                        self.vwin_handlers.push(h.to_string());
                    }
                }
                Ok(vec![Json::Bool(true)])
            }
            "remove-virtual-window-handler" => {
                if let Some(h) = args.first().and_then(|a| a.as_str()) {
                    self.vwin_handlers.retain(|x| x != h);
                }
                Ok(vec![Json::Bool(true)])
            }
            "visible-virtual-window-mouse-click" => {
                let title = args.first().and_then(|a| a.as_str()).unwrap_or("");
                let x = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                let y = args.get(2).and_then(|a| a.as_f64()).unwrap_or(0.0) as i64;
                // Window-relative → global coordinates (`x-pos`/`y-pos`).
                let owner = self.window_owner_by_title(title);
                let (ox, oy) = owner
                    .as_deref()
                    .and_then(|model| self.window_for(model))
                    .map(|w| (w.x, w.y))
                    .unwrap_or((0, 0));
                let target = owner.or_else(|| self.current_model.clone());
                if let Some(eng) = target.as_deref().and_then(|m| self.engine_mut_for(m)) {
                    eng.click_window(ox + x, oy + y);
                }
                Ok(vec![Json::Null])
            }
            "clear-exp-window" => {
                let model = self.current_model.clone().unwrap_or_default();
                if let Some(eng) = &mut self.engine {
                    eng.clear_window();
                }
                let title = self.vwin_title();
                if let Some(w) = self.window_for(&model).cloned() {
                    for key in &w.items {
                        self.vwin_send_to(
                            &model,
                            &[
                                Json::Str("remove".into()),
                                Json::Str(title.clone()),
                                Json::Str(key.clone()),
                            ],
                            remote,
                        );
                    }
                }
                if let Some(w) = self.current_window_mut() {
                    w.items.clear();
                }
                Ok(vec![Json::Null])
            }
            "add-text-to-exp-window" => {
                let text = args.get(1).and_then(|a| a.as_str()).unwrap_or("");
                let opts = args.get(2);
                let (x, y) = (opt_i(opts, "x", 0), opt_i(opts, "y", 0));
                let (w, h) = (opt_i(opts, "width", 75), opt_i(opts, "height", 20));
                let font = opt_i(opts, "font-size", 12);
                let color = opt_str(opts, "color", "black");
                let key = self
                    .engine
                    .as_mut()
                    .map(|e| e.add_window_text_styled(text, x, y, w, h, font, &color))
                    .unwrap_or_default();
                if !key.is_empty() {
                    let model = self.current_model.clone().unwrap_or_default();
                    let title = self.vwin_title();
                    if let Some(win) = self.current_window_mut() {
                        win.items.push(key.clone());
                    }
                    self.vwin_send_to(
                        &model,
                        &[
                            Json::Str("text".into()),
                            Json::Str(title),
                            Json::Str(key.clone()),
                            Json::Int(x),
                            Json::Int(y),
                            Json::Str(text.to_string()),
                            Json::Str(color),
                            Json::Int(font),
                        ],
                        remote,
                    );
                }
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
                if !key.is_empty() {
                    let model = self.current_model.clone().unwrap_or_default();
                    let title = self.vwin_title();
                    if let Some(win) = self.current_window_mut() {
                        win.items.push(key.clone());
                    }
                    self.vwin_send_to(
                        &model,
                        &[
                            Json::Str("line".into()),
                            Json::Str(title),
                            Json::Str(key.clone()),
                            Json::Int(start.0),
                            Json::Int(start.1),
                            Json::Int(end.0),
                            Json::Int(end.1),
                            Json::Str(color.to_string()),
                        ],
                        remote,
                    );
                }
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
                if !key.is_empty() {
                    let model = self.current_model.clone().unwrap_or_default();
                    let title = self.vwin_title();
                    if let Some(win) = self.current_window_mut() {
                        win.items.push(key.clone());
                    }
                    self.vwin_send_to(
                        &model,
                        &[
                            Json::Str("button".into()),
                            Json::Str(title),
                            Json::Str(key.clone()),
                            Json::Int(x),
                            Json::Int(y),
                            Json::Int(w),
                            Json::Int(h),
                            Json::Str(text),
                            Json::Str(color),
                        ],
                        remote,
                    );
                }
                Ok(vec![Json::Str(key)])
            }
            "remove-items-from-exp-window" => {
                let keys: Vec<String> =
                    args[1.min(args.len())..].iter().map(json_to_text).collect();
                let model = self.current_model.clone().unwrap_or_default();
                if let Some(eng) = &mut self.engine {
                    eng.remove_window_items(&keys);
                }
                let title = self.vwin_title();
                for key in &keys {
                    self.vwin_send_to(
                        &model,
                        &[
                            Json::Str("remove".into()),
                            Json::Str(title.clone()),
                            Json::Str(key.clone()),
                        ],
                        remote,
                    );
                }
                if let Some(w) = self.current_window_mut() {
                    w.items.retain(|k| !keys.contains(k));
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
            "install-device" => {
                // Any installed device receives speech notifications; the
                // Python tutorial uses this for `output-speech` monitors.
                if let Some(name) = self.current_model.clone() {
                    self.speech_models.insert(name);
                }
                if let Some(eng) = &mut self.engine {
                    eng.set_speech_device(true);
                }
                Ok(vec![Json::Null])
            }
            "visible-virtuals-available?" => Ok(vec![Json::Bool(!self.vwin_handlers.is_empty())]),
            "start-hand-at-mouse" | "process-events" => Ok(vec![Json::Null]),
            // Window key presses are forwarded by the Environment; the Rust
            // Window key presses are forwarded by the Environment; the Rust
            // task layer has no Lisp output-key handler to invoke, so accept
            // the command (monitors still fire through the command path).
            "output-key" | "output-speech" | "release-key" => Ok(vec![Json::Bool(true)]),
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
            "whynot-dm" => {
                let names: Vec<String> = args
                    .iter()
                    .filter_map(|a| a.as_str().map(str::to_string))
                    .collect();
                if let Some(eng) = &mut self.engine {
                    eng.print_whynot_dm(&names);
                }
                Ok(vec![Json::Null])
            }
            "print-conflict-tree" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_conflict_tree();
                }
                Ok(vec![Json::Null])
            }
            "simulate-retrieval-request" => {
                let forms: Vec<Sexp> = args.iter().map(json_to_sexp).collect();
                let desc = crate::model::parse_chunk_desc(&forms).map_err(|e| e.to_string())?;
                if let Some(eng) = &mut self.engine {
                    eng.simulate_retrieval(&desc).map_err(|e| e.to_string())?;
                }
                Ok(vec![Json::Null])
            }
            "print-chunk-activation-trace" => {
                let name = args
                    .first()
                    .and_then(|a| a.as_str())
                    .ok_or("missing chunk name")?;
                let raw = args.get(1).and_then(|a| a.as_f64()).unwrap_or(0.0);
                let in_ms = !matches!(args.get(2), Some(Json::Bool(false)) | Some(Json::Null));
                let time = if in_ms {
                    raw as i64
                } else {
                    (raw * 1000.0).round() as i64
                };
                if let Some(eng) = &mut self.engine {
                    eng.print_chunk_activation_trace(name, time);
                }
                Ok(vec![Json::Null])
            }
            "print-activation-trace" => {
                let raw = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
                let in_ms = !matches!(args.get(1), Some(Json::Bool(false)) | Some(Json::Null));
                let time = if in_ms {
                    raw as i64
                } else {
                    (raw * 1000.0).round() as i64
                };
                if let Some(eng) = &mut self.engine {
                    eng.print_activation_trace(time);
                }
                Ok(vec![Json::Null])
            }
            "print-dm-finsts" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_dm_finsts();
                }
                Ok(vec![Json::Null])
            }
            "mp-show-queue" => {
                if let Some(eng) = &mut self.engine {
                    eng.print_queue();
                }
                Ok(vec![Json::Null])
            }
            "mp-queue-text" => {
                let Some(eng) = self.engine.as_mut() else {
                    return Ok(vec![Json::Str(String::new())]);
                };
                let mark = eng.out_len();
                eng.print_queue();
                let text = eng.out_text_since(mark);
                eng.truncate_out(mark);
                if self.trace_pos > mark {
                    self.trace_pos = mark;
                }
                Ok(vec![Json::Str(text)])
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
        // The Environment's single-step (one event, no terminator).
        if name == "step-once" {
            return self.step_once().map(|b| vec![Json::Bool(b)]);
        }
        let start_ms = (self.time_secs() * 1000.0).round() as i64;
        let mut break_hit = false;
        if self.is_multi() && name == "run" {
            self.run_multi_secs(secs, true, crate::trace::MODULE_WIDTH)?;
        } else {
            let eng = self.engine.as_mut().ok_or("no model loaded")?;
            let r = match name {
                "run" => eng.run_secs(secs),
                "run-full-time" => eng.run_full_time(secs),
                "run-until-time" => eng.run_until(secs),
                "run-n-events" => eng.run_n_events(secs.max(0.0) as u64),
                _ => unreachable!(),
            };
            r.map_err(|e: EngineError| e.to_string())?;
            break_hit = eng.stopped();
        }
        // `run` returns `(values seconds events break)` in ACT-R; the first is
        // the elapsed model time (not the absolute clock), the second the
        // event count (`scheduling.lisp:877`), and the third the break flag.
        let elapsed_ms = (self.time_secs() * 1000.0).round() as i64 - start_ms;
        Ok(vec![
            Json::Float(elapsed_ms as f64 / 1000.0),
            Json::Int(self.run_events() as i64),
            if break_hit {
                Json::Bool(true)
            } else {
                Json::Null
            },
        ])
    }

    fn stepper_id_matches(&self, id: &str) -> bool {
        self.stepper
            .as_ref()
            .is_some_and(|s| s.id.eq_ignore_ascii_case(id))
    }

    fn start_stepper(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        if self.stepper.is_some() {
            return Ok(vec![
                Json::Bool(false),
                Json::Str("There is already a stepper open".into()),
            ]);
        }
        let step_all = matches!(args.first(), Some(Json::Bool(true)));
        let tutoring = matches!(args.get(1), Some(Json::Bool(true)));
        let next_cmd = args
            .get(2)
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();
        let update_cmd = args
            .get(3)
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .to_string();
        if !self.has_command(&next_cmd) {
            return Ok(vec![
                Json::Bool(false),
                Json::Str(format!(
                    "The next-event-fn given {next_cmd:?} did not name a valid command."
                )),
            ]);
        }
        if !self.has_command(&update_cmd) {
            return Ok(vec![
                Json::Bool(false),
                Json::Str(format!(
                    "The update-fn given {update_cmd:?} did not name a valid command."
                )),
            ]);
        }
        if !step_all {
            let trace_on = self
                .engine
                .iter()
                .chain(self.inactive.iter().map(|m| &m.engine))
                .any(|e| e.get_param(":V").as_deref() != Some("NIL"));
            let trace_off = !trace_on;
            if trace_off {
                return Ok(vec![
                    Json::Bool(false),
                    Json::Str("Step-all not enabled and all models have :v set to nil.".into()),
                ]);
            }
        }
        self.symbol_count += 1;
        let id = format!("STEPPER{}", self.symbol_count);
        let shared = Rc::new(RefCell::new(StepperShared {
            step_all,
            tutoring,
            ..StepperShared::default()
        }));
        if let Some(eng) = &mut self.engine {
            eng.set_event_hook(Some(Box::new(StepperHook {
                shared: shared.clone(),
                model: self.current_model.clone().unwrap_or_default(),
            })));
        }
        for model in &mut self.inactive {
            model.engine.set_event_hook(Some(Box::new(StepperHook {
                shared: shared.clone(),
                model: model.name.clone(),
            })));
        }
        self.stepper = Some(Stepper {
            id: id.clone(),
            next_cmd,
            update_cmd,
            shared,
            snapshot: None,
            last_action: String::new(),
        });
        Ok(vec![Json::Bool(true), Json::Str(id)])
    }

    fn stop_stepper(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let id = args.first().and_then(|a| a.as_str()).unwrap_or("");
        if self.stepper_id_matches(id) {
            self.stepper = None;
            if let Some(eng) = &mut self.engine {
                eng.set_event_hook(None);
            }
            for model in &mut self.inactive {
                model.engine.set_event_hook(None);
            }
            Ok(vec![Json::Bool(true)])
        } else {
            Ok(vec![Json::Bool(false)])
        }
    }

    fn step_stepper(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let id = args.first().and_then(|a| a.as_str()).unwrap_or("");
        if !self.stepper_id_matches(id) {
            return Ok(vec![
                Json::Bool(false),
                Json::Str("Invalid id provided to stepper-step".into()),
            ]);
        }
        let stop = matches!(args.get(1), Some(Json::Bool(true)));
        if stop {
            let stepper = self.stepper.as_ref().unwrap();
            let mut s = stepper.shared.borrow_mut();
            s.abort_run = true;
            s.paused = None;
            s.pause_reported = false;
            s.approved = false;
            return Ok(vec![Json::Bool(true), Json::Str(String::new())]);
        }
        let kind = args
            .get(2)
            .and_then(|a| a.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase());
        let value = args.get(3);
        let mut skip_type = None;
        let mut skip_value = None;
        let mut skip_time_ms = None;
        if let Some(kind) = &kind {
            match kind.as_str() {
                "production" => {
                    let val = value.and_then(|v| v.as_str()).unwrap_or("");
                    let found = self.engine.as_ref().is_some_and(|e| {
                        e.all_production_names()
                            .iter()
                            .any(|n| n.eq_ignore_ascii_case(val))
                    });
                    if val.is_empty() || !found {
                        return Ok(vec![
                            Json::Bool(false),
                            Json::Str(
                                "Step type Production requires a valid production name.".into(),
                            ),
                        ]);
                    }
                    skip_type = Some("production".to_string());
                    skip_value = Some(val.to_uppercase());
                }
                "time" => {
                    let secs = value.and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()));
                    let now = self.time_secs();
                    match secs {
                        Some(t) if t > now => {
                            skip_type = Some("time".to_string());
                            skip_time_ms = Some((t * 1000.0).round() as i64);
                        }
                        Some(_) => {
                            return Ok(vec![
                                Json::Bool(false),
                                Json::Str(
                                    "Step type Time given a time that has already occurred.".into(),
                                ),
                            ])
                        }
                        None => {
                            return Ok(vec![
                                Json::Bool(false),
                                Json::Str("Step type Time requires a number in seconds.".into()),
                            ])
                        }
                    }
                }
                "module" => {
                    let val = value.and_then(|v| v.as_str()).unwrap_or("");
                    let found = self.engine.as_ref().is_some_and(|e| {
                        e.module_names().iter().any(|m| m.eq_ignore_ascii_case(val))
                    });
                    if !found {
                        return Ok(vec![
                            Json::Bool(false),
                            Json::Str("Step type Module requires a valid module name.".into()),
                        ]);
                    }
                    skip_type = Some("module".to_string());
                    skip_value = Some(val.to_uppercase());
                }
                other => {
                    return Ok(vec![
                        Json::Bool(false),
                        Json::Str(format!("Step type {other:?} not a valid type.")),
                    ])
                }
            }
        }
        let stepper = self.stepper.as_ref().unwrap();
        let mut s = stepper.shared.borrow_mut();
        let was_paused = s.paused.is_some();
        s.skip_type = skip_type;
        s.skip_value = skip_value;
        s.skip_time_ms = skip_time_ms;
        s.abort_run = false;
        if was_paused {
            s.approved = true;
            s.paused = None;
            s.pause_reported = false;
            s.pending_step_desc = s.pause_desc.take();
        }
        Ok(vec![Json::Bool(true), Json::Str(String::new())])
    }

    fn update_stepper(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let item = args.first().and_then(|a| a.as_str());
        let (snapshot, action, tutoring) = match &self.stepper {
            Some(st) => (
                st.snapshot.clone(),
                st.last_action.clone(),
                st.shared.borrow().tutoring,
            ),
            None => (None, String::new(), false),
        };
        let Some(snapshot) = snapshot else {
            return Ok(if item.is_some() {
                vec![
                    Json::Str(String::new()),
                    Json::Str(String::new()),
                    Json::Str(String::new()),
                    Json::Null,
                ]
            } else {
                vec![Json::Null]
            });
        };
        if let Some(item) = item {
            let Some(d) = snapshot
                .details
                .iter()
                .find(|d| d.name.eq_ignore_ascii_case(item))
            else {
                return Ok(vec![
                    Json::Str(String::new()),
                    Json::Str(String::new()),
                    Json::Str(String::new()),
                    Json::Null,
                ]);
            };
            let production = snapshot
                .labels
                .first()
                .is_some_and(|l| l == "Possible Productions");
            if production {
                let display = if action == "production-selected" || tutoring {
                    d.printed.clone()
                } else {
                    d.instantiation.clone()
                };
                let rows: Vec<Json> = d
                    .binding_details
                    .iter()
                    .map(|(var, val, is_buf)| {
                        Json::Arr(vec![
                            Json::Str(var.clone()),
                            Json::Str(val.clone()),
                            Json::Bool(*is_buf),
                        ])
                    })
                    .collect();
                Ok(vec![
                    Json::Str(d.params.clone()),
                    Json::Str(d.bindings.clone()),
                    Json::Str(display),
                    Json::Arr(rows),
                ])
            } else {
                Ok(vec![
                    Json::Str(d.params.clone()),
                    Json::Str(d.bindings.clone()),
                    Json::Str(d.printed.clone()),
                    Json::Null,
                ])
            }
        } else {
            Ok(vec![snapshot.tutoring])
        }
    }

    fn set_stepper_ready(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let id = args.first().and_then(|a| a.as_str()).unwrap_or("");
        if let Some(st) = &self.stepper {
            if st.id.eq_ignore_ascii_case(id) {
                st.shared.borrow_mut().ready = true;
                return Ok(vec![Json::Bool(true)]);
            }
        }
        Ok(vec![Json::Bool(false)])
    }

    fn set_stepper_tutoring(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let id = args.first().and_then(|a| a.as_str()).unwrap_or("");
        let value = matches!(args.get(1), Some(Json::Bool(true)));
        if let Some(st) = &self.stepper {
            if st.id.eq_ignore_ascii_case(id) {
                st.shared.borrow_mut().tutoring = value;
                return Ok(vec![Json::Bool(true)]);
            }
        }
        Ok(vec![Json::Bool(false)])
    }

    fn set_stepper_step_all(&mut self, args: &[Json]) -> Result<Vec<Json>, String> {
        let id = args.first().and_then(|a| a.as_str()).unwrap_or("");
        let value = matches!(args.get(1), Some(Json::Bool(true)));
        if let Some(st) = &self.stepper {
            if st.id.eq_ignore_ascii_case(id) {
                st.shared.borrow_mut().step_all = value;
                return Ok(vec![Json::Bool(true)]);
            }
        }
        Ok(vec![Json::Bool(false)])
    }

    /// True while a stepper is paused waiting for `step-stepper`.
    pub fn stepper_paused(&self) -> bool {
        self.stepper
            .as_ref()
            .is_some_and(|s| s.shared.borrow().paused.is_some())
    }

    /// Consume the `step-stepper` Stop button's request to abort the run.
    pub fn take_stepper_abort(&self) -> bool {
        match &self.stepper {
            Some(s) => {
                let mut shared = s.shared.borrow_mut();
                let abort = shared.abort_run;
                shared.abort_run = false;
                abort
            }
            None => false,
        }
    }

    /// Return the next-event callback and its formatted event argument once
    /// per paused event.
    pub fn take_stepper_pause(&mut self) -> Option<(String, Option<String>, Vec<Json>)> {
        let (next_cmd, shared) = {
            let st = self.stepper.as_ref()?;
            (st.next_cmd.clone(), st.shared.clone())
        };
        let model = {
            let mut s = shared.borrow_mut();
            if s.paused.is_none() || s.pause_reported {
                return None;
            }
            s.pause_reported = true;
            s.paused_model.clone()
        };
        let (_, description) = self
            .engine_mut_for(model.as_deref().unwrap_or(""))?
            .next_event()?;
        shared.borrow_mut().pause_desc = Some(description.clone());
        Some((next_cmd, model, vec![Json::Str(description)]))
    }

    /// Return the update callback and its seven arguments once the approved
    /// event has executed.
    pub fn take_stepper_update(&mut self) -> Option<(String, Option<String>, Vec<Json>)> {
        let (update_cmd, model, view, description) = {
            let st = self.stepper.as_ref()?;
            let shared = st.shared.clone();
            let mut s = shared.borrow_mut();
            if !s.update_ready {
                return None;
            }
            s.update_ready = false;
            let view = s.stepped.take()?;
            let model = s.stepped_model.clone();
            let description = s.last_step_desc.take().unwrap_or_default();
            (st.update_cmd.clone(), model, view, description)
        };
        let snapshot = self
            .engine_mut_for(model.as_deref().unwrap_or(""))?
            .stepper_snapshot(&view);
        let items = if snapshot.items.is_empty() {
            Json::Null
        } else {
            Json::Arr(snapshot.items.iter().cloned().map(Json::Str).collect())
        };
        let tutorable = snapshot
            .tutorable
            .clone()
            .map(Json::Str)
            .unwrap_or(Json::Null);
        let mut args = vec![Json::Str(description), items, tutorable];
        args.extend(snapshot.labels.iter().cloned().map(Json::Str));
        if let Some(st) = self.stepper.as_mut() {
            st.snapshot = Some(snapshot);
            st.last_action = view.action.clone();
        }
        Some((update_cmd, model, args))
    }

    /// Interleaved-run support for the server: prepare the absolute limit and
    /// the hidden dummy (`run_begin`), then drive the run in slices so a click
    /// can dispatch its client-owned `:action`.
    pub fn run_begin(&mut self, name: &str, secs: f64) -> (i64, bool, bool) {
        if self.is_multi() {
            let limit = self.global_time_ms + (secs * 1000.0).round() as i64;
            let any = self
                .engine
                .as_mut()
                .and_then(|e| e.next_event_key())
                .is_some()
                || self
                    .inactive
                    .iter_mut()
                    .any(|m| m.engine.next_event_key().is_some());
            return (limit, false, any);
        }
        self.engine
            .as_mut()
            .map(|e| e.run_begin(name, secs))
            .unwrap_or((0, false, false))
    }

    /// One run slice, reporting why it returned.
    pub fn run_slice_reason(&mut self, limit_ms: i64, full: bool) -> Result<RunStop, String> {
        if self.is_multi() {
            return self.run_multi_slice(limit_ms, None);
        }
        if self
            .engine
            .as_ref()
            .is_some_and(Engine::has_pending_device_actions)
        {
            return Ok(RunStop::DeviceAction);
        }
        let device = self
            .engine
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?
            .run_slice(limit_ms, full)
            .map_err(|e| e.to_string())?;
        if device {
            return Ok(RunStop::DeviceAction);
        }
        if self.stepper_paused() {
            return Ok(RunStop::StepperPause);
        }
        let update = self
            .stepper
            .as_ref()
            .is_some_and(|s| s.shared.borrow().update_ready);
        if update {
            Ok(RunStop::StepperUpdate)
        } else {
            Ok(RunStop::Finished)
        }
    }

    /// One capped run slice for the server loop. A single model processes at
    /// most `max_events` events so the connection can service Stop and other
    /// client requests between slices; multi-model already advances one event
    /// per call.
    pub fn run_slice_reason_capped(
        &mut self,
        limit_ms: i64,
        full: bool,
        max_events: u64,
    ) -> Result<RunStop, String> {
        if self.is_multi() {
            return self.run_multi_slice(limit_ms, Some(1));
        }
        if self
            .engine
            .as_ref()
            .is_some_and(Engine::has_pending_device_actions)
        {
            return Ok(RunStop::DeviceAction);
        }
        let still = self
            .engine
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?
            .run_chunk(limit_ms, full, Some(max_events), None)
            .map_err(|e| e.to_string())?;
        if self
            .engine
            .as_ref()
            .is_some_and(Engine::has_pending_device_actions)
        {
            return Ok(RunStop::DeviceAction);
        }
        if self.stepper_paused() {
            return Ok(RunStop::StepperPause);
        }
        if self.stepper_update_ready() {
            return Ok(RunStop::StepperUpdate);
        }
        if still {
            Ok(RunStop::Continue)
        } else {
            Ok(RunStop::Finished)
        }
    }

    fn stepper_update_ready(&self) -> bool {
        self.stepper
            .as_ref()
            .is_some_and(|s| s.shared.borrow().update_ready)
    }

    pub fn stepper_active(&self) -> bool {
        self.stepper.is_some()
    }

    fn run_multi_slice(
        &mut self,
        limit_ms: i64,
        max_events: Option<u64>,
    ) -> Result<RunStop, String> {
        let model_width =
            crate::engine::multi::model_column_width(self.model_order.iter().map(String::len));
        let mut processed = 0u64;
        loop {
            let pending_action = self
                .engine
                .as_ref()
                .is_some_and(Engine::has_pending_device_actions)
                || self
                    .inactive
                    .iter()
                    .any(|m| m.engine.has_pending_device_actions());
            if pending_action {
                return Ok(RunStop::DeviceAction);
            }
            if self.stepper_paused() {
                return Ok(RunStop::StepperPause);
            }
            if self.stepper_update_ready() {
                return Ok(RunStop::StepperUpdate);
            }
            let mut best: Option<(i64, i64, u64, usize)> = None;
            let mut beyond_limit = false;
            if let Some((time, priority, seq)) =
                self.engine.as_mut().and_then(|e| e.next_event_key())
            {
                if time > limit_ms {
                    beyond_limit = true;
                } else {
                    best = Some((time, priority, seq, 0));
                }
            }
            for (i, model) in self.inactive.iter_mut().enumerate() {
                if let Some((time, priority, seq)) = model.engine.next_event_key() {
                    if time > limit_ms {
                        beyond_limit = true;
                        continue;
                    }
                    let key = (time, priority, seq, i + 1);
                    if best.map_or(true, |b| {
                        (key.0, std::cmp::Reverse(key.1), key.2)
                            < (b.0, std::cmp::Reverse(b.1), b.2)
                    }) {
                        best = Some(key);
                    }
                }
            }
            let Some((_, _, _, idx)) = best else {
                let stop = if beyond_limit {
                    limit_ms
                } else {
                    self.engine
                        .as_ref()
                        .map(Engine::time_ms)
                        .into_iter()
                        .chain(self.inactive.iter().map(|m| m.engine.time_ms()))
                        .max()
                        .unwrap_or(self.global_time_ms)
                };
                self.global_time_ms = stop;
                for engine in self
                    .engine
                    .iter_mut()
                    .chain(self.inactive.iter_mut().map(|m| &mut m.engine))
                {
                    engine.set_time_ms(stop);
                }
                let reason = if beyond_limit {
                    "Stopped because time limit reached"
                } else {
                    "Stopped because no events left to process"
                };
                let raw =
                    crate::trace::format_line(stop, "------", reason, crate::trace::MODULE_WIDTH);
                self.global_trace
                    .push(crate::engine::multi::with_model_column(
                        &raw,
                        "-",
                        model_width,
                    ));
                self.global_trace_kinds.push(TraceKind::Model);
                self.global_trace_levels.push(Out::Low);
                return Ok(RunStop::Finished);
            };
            let model_name = if idx == 0 {
                self.current_model.clone().unwrap_or_default()
            } else {
                self.inactive[idx - 1].name.clone()
            };
            let (time, lines, kinds, levels, stopped) = if idx == 0 {
                let eng = self.engine.as_mut().ok_or("no model loaded")?;
                let mark = eng.out_len();
                if !eng.run_one_event().map_err(|e| e.to_string())? {
                    return Ok(RunStop::StepperPause);
                }
                (
                    eng.time_ms(),
                    eng.out_since(mark),
                    eng.out_kinds_since(mark),
                    eng.out_levels_since(mark),
                    eng.stopped(),
                )
            } else {
                let eng = &mut self.inactive[idx - 1].engine;
                let mark = eng.out_len();
                if !eng.run_one_event().map_err(|e| e.to_string())? {
                    return Ok(RunStop::StepperPause);
                }
                (
                    eng.time_ms(),
                    eng.out_since(mark),
                    eng.out_kinds_since(mark),
                    eng.out_levels_since(mark),
                    eng.stopped(),
                )
            };
            for (i, line) in lines.into_iter().enumerate() {
                self.global_trace
                    .push(crate::engine::multi::with_model_column(
                        &line,
                        &model_name,
                        model_width,
                    ));
                self.global_trace_kinds
                    .push(kinds.get(i).copied().unwrap_or(TraceKind::Model));
                self.global_trace_levels
                    .push(levels.get(i).copied().unwrap_or(Out::Low));
            }
            self.global_time_ms = self.global_time_ms.max(time);
            self.global_run_events += 1;
            if stopped {
                // A break event is global to the meta-process: stop every
                // model and emit the `run` terminator once.
                let stop = self.global_time_ms;
                let raw = crate::trace::format_line(
                    stop,
                    "------",
                    "Stopped because break event",
                    crate::trace::MODULE_WIDTH,
                );
                self.global_trace
                    .push(crate::engine::multi::with_model_column(
                        &raw,
                        "-",
                        model_width,
                    ));
                self.global_trace_kinds.push(TraceKind::Model);
                self.global_trace_levels.push(Out::Low);
                for engine in self
                    .engine
                    .iter_mut()
                    .chain(self.inactive.iter_mut().map(|m| &mut m.engine))
                {
                    engine.set_time_ms(stop);
                }
                return Ok(RunStop::Finished);
            }
            processed += 1;
            if max_events.is_some_and(|n| processed >= n) {
                return Ok(RunStop::Continue);
            }
        }
    }

    /// One run slice; `true` when it yielded for a pending device action.
    pub fn run_slice(&mut self, limit_ms: i64, full: bool) -> Result<bool, String> {
        Ok(matches!(
            self.run_slice_reason(limit_ms, full)?,
            RunStop::DeviceAction
        ))
    }

    /// One chunk of an interactive run (Environment): process toward
    /// `limit_ms`, capped by `max_events`/`soft_ms`, returning whether the run
    /// is still going. See [`Engine::run_chunk`].
    pub fn run_chunk(
        &mut self,
        limit_ms: i64,
        full: bool,
        max_events: Option<u64>,
        soft_ms: Option<i64>,
    ) -> Result<bool, String> {
        self.engine
            .as_mut()
            .ok_or_else(|| "no model loaded".to_string())?
            .run_chunk(limit_ms, full, max_events, soft_ms)
            .map_err(|e| e.to_string())
    }

    /// The Environment's single-step: execute one event (no stop terminator).
    pub fn step_once(&mut self) -> Result<bool, String> {
        self.engine
            .as_mut()
            .map(|e| e.step_once().map_err(|err| err.to_string()))
            .unwrap_or(Ok(false))
    }

    /// Drain the selected model's device actions, for the Python bridge.
    pub fn take_pending_device_actions(&mut self) -> Vec<(String, Vec<Json>)> {
        self.engine
            .as_mut()
            .map(|e| e.take_pending_device_actions())
            .unwrap_or_default()
    }

    /// Drain device actions from every model, tagging each with its owner.
    pub fn take_pending_device_actions_with_models(
        &mut self,
    ) -> Vec<(Option<String>, String, Vec<Json>)> {
        let mut actions = Vec::new();
        if let Some(engine) = &mut self.engine {
            let model = self.current_model.clone();
            for (command, args) in engine.take_pending_device_actions() {
                actions.push((model.clone(), command, args));
            }
        }
        for model in &mut self.inactive {
            for (command, args) in model.engine.take_pending_device_actions() {
                actions.push((Some(model.name.clone()), command, args));
            }
        }
        actions
    }

    /// The model time in seconds, for `run`'s first return value.
    pub fn time_secs(&self) -> f64 {
        if self.is_multi() {
            self.global_time_ms as f64 / 1000.0
        } else {
            self.engine.as_ref().map(|e| e.time_secs()).unwrap_or(0.0)
        }
    }

    /// Events evaluated in the last `run`, for its second return value.
    pub fn run_events(&self) -> u64 {
        if self.is_multi() {
            self.global_run_events
        } else {
            self.engine.as_ref().map(|e| e.run_events()).unwrap_or(0)
        }
    }

    /// Whether the last run stopped on a break event.
    pub fn run_break_hit(&self) -> bool {
        self.engine.as_ref().is_some_and(|e| e.stopped())
    }

    /// Load model definitions from in-memory Lisp source.
    ///
    /// This is the embedder entry point: a Python builder renders a
    /// `define-model` source and hands it here instead of writing a file.
    /// Task-file `defun`s are not parsed; Python callbacks are registered
    /// through [`Session::add_remote`] and the fired-command bridge.
    pub fn load_source(&mut self, source: &str) -> Result<(), String> {
        let loaded = crate::model::load_models_from_str(source).map_err(|e| e.to_string())?;
        self.prepare_model_load();
        for def in loaded {
            self.add_model_definition(def, None)?;
        }
        self.model_files.push("<source>".to_string());
        Ok(())
    }

    /// Load a model or task file from a filesystem path.  ACT-R logical names
    /// (`ACT-R:tutorial;unit1;count.lisp`) are deliberately unsupported.
    fn load(&mut self, path: &str, remote: &mut dyn Remote) -> Result<(), String> {
        self.load_file(path, remote, true).map(|_| ())
    }

    /// Load a file and optionally dispatch its `creating-model` signal. Reload
    /// suppresses the immediate signal so it can emit delete/create in order.
    fn load_file(
        &mut self,
        path: &str,
        remote: &mut dyn Remote,
        notify: bool,
    ) -> Result<Vec<String>, String> {
        let path = filesystem_path(path)?;
        if !path.is_file() {
            return Err(format!("no such file: {}", path.display()));
        }
        let mut created = Vec::new();
        let source = std::fs::read_to_string(&path).ok();
        if let Some(src) = &source {
            self.user_fns.extend(model::parse_defuns(src));
            self.user_vars.extend(model::parse_task_globals(src));
            for (cmd, fname) in model::parse_commands(src) {
                if let Some(def) = self.user_fns.get(&fname).cloned() {
                    self.user_fns.insert(cmd, def);
                }
            }
        }
        if !crate::file_defines_model(&path)? {
            // SBCL's `load` evaluates ordinary top-level forms, so a code
            // sidecar can use the existing `(delete-model name)` form even
            // though it is not a remote command.
            if let Some(src) = &source {
                for target in model::parse_model_deletions(src) {
                    let name = target.or_else(|| self.current_model.clone());
                    let Some(name) = name else {
                        continue;
                    };
                    self.queue_signal("deleting-model", vec![Json::Str(name.clone())]);
                    self.close_model_window(&name, remote);
                    self.remove_model(&name);
                }
            }
        }
        if crate::file_defines_model(&path)? {
            self.prepare_model_load();
            let loaded = load_models(&path).map_err(|e| e.to_string())?;
            for def in loaded {
                let name = def.name.clone();
                self.close_model_window(&name, remote);
                created.push(name);
                self.add_model_definition(def, Some(path.to_string_lossy().into_owned()))?;
            }
            self.last_model_file = Some(path.to_string_lossy().into_owned());
        }
        self.model_files.push(path.to_string_lossy().into_owned());
        // `add-new-model` dispatches `creating-model` so the Environment's
        // model menu updates (`meta-process.lisp:426`).
        if notify {
            for name in &created {
                let _ = self.evaluate(
                    "creating-model",
                    Some(name),
                    &[Json::Str(name.clone())],
                    remote,
                );
            }
        }
        Ok(created)
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
            // reset hook invokes it separately, PORTING-NOTES #219).
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
    /// `reset` command takes (`invoke_testing_fn`). The test harness replays a
    /// spec's `(:RESET)` through this rather than the engine directly, or a spec
    /// whose runs depend on the hook (the fan stubs' `trace-fan-run`, which sets
    /// `:act t`) silently loses the trace it enables.
    pub fn reset_model(&mut self) -> Result<(), String> {
        if let Some(eng) = &mut self.engine {
            eng.reset().map_err(|e| e.to_string())?;
        }
        struct NoRemote;
        impl Remote for NoRemote {
            fn call(
                &mut self,
                _n: &str,
                _m: Option<&str>,
                _a: Vec<Json>,
            ) -> Result<Vec<Json>, String> {
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

    /// Signals a command generated for the server to deliver after it has
    /// released the session borrow.
    pub fn take_pending_signals(&mut self) -> Vec<(String, Option<String>, Vec<Json>)> {
        std::mem::take(&mut self.pending_signals)
    }

    fn queue_signal(&mut self, name: &str, args: Vec<Json>) {
        self.pending_signals
            .push((name.to_string(), self.current_model.clone(), args));
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
        // `break-fan-run` (`test-models/fan.lisp:4`): testing traces plus a
        // breakpoint on ENCODE-PERSON, which prints the instantiation and stops
        // the run so the caller can dump the retrieval buffer.
        if name.eq_ignore_ascii_case("break-fan-run") {
            if let Some(eng) = &mut self.engine {
                let _ = eng.apply_sgp(":v", &Sexp::Sym("T".into()));
                let _ = eng.apply_sgp(":cmdt", &Sexp::Sym("T".into()));
                eng.pbreak(&["ENCODE-PERSON".to_string()]);
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
        let sexps: Vec<Sexp> = match options {
            // The test-step command form has no JSON object type, so a flat
            // third argument is its parameter list.
            Some(Json::Arr(a)) => a.iter().map(json_to_sexp).collect(),
            Some(Json::Obj(items)) => match items.iter().find(|(k, _)| k == "params") {
                Some((_, Json::Arr(a))) => a.iter().map(json_to_sexp).collect(),
                _ => Vec::new(),
            },
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
        // `build_engine` also resets the global trace buffer. A runtime
        // definition change (the relay's `define-chunks`, a task's
        // `chunk-type`) must not replay or reorder the trace already
        // delivered this run.
        let global_trace = std::mem::take(&mut self.global_trace);
        let global_kinds = std::mem::take(&mut self.global_trace_kinds);
        let global_levels = std::mem::take(&mut self.global_trace_levels);
        let global_pos = self.global_trace_pos;
        let trace_pos = self.trace_pos;
        let result = self.build_engine();
        self.global_trace = global_trace;
        self.global_trace_kinds = global_kinds;
        self.global_trace_levels = global_levels;
        self.global_trace_pos = global_pos;
        self.trace_pos = trace_pos;
        result?;
        if let Some(eng) = self.engine.as_mut() {
            eng.set_device_run(run);
            eng.set_out_lines(trace);
        }
        Ok(())
    }

    /// `schedule-break[-relative]`: enqueue `Event::Break`. The Tcl
    /// Environment's Stop button sends `schedule-break-relative 0`; other
    /// callers may pass `:priority :min` and details.
    fn schedule_break_cmd(&mut self, relative: bool, args: &[Json]) {
        let Some(time) = args.first().and_then(|a| {
            a.as_f64()
                .or_else(|| a.as_str().and_then(|s| s.parse::<f64>().ok()))
        }) else {
            return;
        };
        let priority_min = args.iter().any(|a| {
            a.as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case(":min") || s.eq_ignore_ascii_case("min"))
        });
        let details = args
            .iter()
            .skip(1)
            .filter_map(|a| a.as_str())
            .find(|s| {
                !s.starts_with(':')
                    && !s.eq_ignore_ascii_case("min")
                    && !s.eq_ignore_ascii_case("max")
            })
            .unwrap_or("")
            .to_string();
        let now_ms = (self.time_secs() * 1000.0).round() as i64;
        let target_ms = if relative {
            now_ms + (time * 1000.0).round() as i64
        } else {
            (time * 1000.0).round() as i64
        };
        let delay_ms = (target_ms - now_ms).max(0);
        if let Some(eng) = &mut self.engine {
            eng.schedule_break_relative(delay_ms, priority_min, details);
        }
    }

    fn build_engine(&mut self) -> Result<(), String> {
        let def = if let Some(name) = self.current_model.as_deref() {
            self.defs
                .iter()
                .find(|d| d.name.eq_ignore_ascii_case(name))
                .cloned()
        } else {
            self.defs.last().cloned()
        };
        let Some(def) = def else {
            return Ok(());
        };
        self.current_model = Some(def.name.clone());
        let eng = self.make_engine(&def)?;
        self.engine = Some(eng);
        self.rebuild_global_trace();
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

/// The trace stream a command's own output belongs to.
fn command_trace_kind(name: &str) -> TraceKind {
    match name {
        "run" | "run-full-time" | "run-until-time" | "run-n-events" | "step-once" | "reset"
        | "reload" | "load-act-r-model" | "load-act-r-code" | "clear-all" => TraceKind::Model,
        "print-warning" => TraceKind::Warning,
        "act-r-output" => TraceKind::General,
        "model-output" => TraceKind::Model,
        _ => TraceKind::Command,
    }
}

/// Runtime model paths are filesystem paths.  ACT-R's logical names are a
/// fixture/tooling concern, not part of the Rust runtime.
fn filesystem_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim_start();
    if trimmed.to_ascii_uppercase().starts_with("ACT-R:") || trimmed.contains(';') {
        return Err(format!(
            "ACT-R logical paths are not supported: {path:?}; pass a filesystem path"
        ));
    }
    Ok(PathBuf::from(path))
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
/// A boolean option (`:visible`): absent → `default`; `nil`/`false` → false.
fn opt_bool(options: Option<&Json>, key: &str, default: bool) -> bool {
    match option(options, key) {
        None => default,
        Some(Json::Bool(b)) => *b,
        Some(Json::Null) => false,
        Some(Json::Str(s)) => !s.eq_ignore_ascii_case("nil") && !s.eq_ignore_ascii_case("false"),
        Some(Json::Int(i)) => *i != 0,
        Some(_) => true,
    }
}

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

/// The Environment's multi-model trace column starts at byte 12.
fn trace_model_from_line(line: &str) -> Option<String> {
    if line.len() <= 12 || line.as_bytes().get(6) != Some(&b'.') {
        return None;
    }
    let model = line.get(12..)?.split_whitespace().next()?;
    (!model.is_empty()).then(|| model.to_string())
}

fn trace_detail_name(level: Out) -> &'static str {
    match level {
        Out::Low => "low",
        Out::Medium => "medium",
        Out::Always | Out::Never => "high",
    }
}

/// One `trace-history` record: `[time, detail, model, text]`. The
/// Environment's text-trace window reads index 3 (`61-trace-history.tcl`).
fn trace_record(record: &TraceHistoryRecord) -> String {
    let time = record
        .line
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|s| (s * 1000.0).round() as i64)
        .unwrap_or(0);
    Json::Arr(vec![
        Json::Int(time),
        Json::Str(trace_detail_name(record.level).into()),
        Json::Str(record.model.clone()),
        Json::Str(record.line.clone()),
    ])
    .write()
}

fn history_time_arg(arg: Option<&Json>) -> Option<i64> {
    arg.and_then(|a| a.as_f64())
        .map(|s| (s * 1000.0).round() as i64)
}

fn record_time_ms(record: &str) -> Option<i64> {
    let v = crate::json::parse(record).ok()?;
    let Json::Arr(items) = v else {
        return None;
    };
    items
        .first()
        .and_then(|t| t.as_f64())
        .map(|t| t.round() as i64)
}

fn trace_detail_allows(record: &str, requested: &str) -> bool {
    let requested = requested.trim().to_ascii_lowercase();
    if requested.is_empty()
        || requested == "nil"
        || requested == "null"
        || requested == "t"
        || requested == "true"
        || requested == "all"
    {
        return true;
    }
    let Ok(Json::Arr(items)) = crate::json::parse(record) else {
        return true;
    };
    let detail = items
        .get(1)
        .and_then(|v| v.as_str())
        .unwrap_or("low")
        .to_ascii_lowercase();
    match requested.as_str() {
        "low" => detail == "low",
        "medium" => matches!(detail.as_str(), "low" | "medium"),
        "high" => true,
        _ => true,
    }
}

fn filter_history_records(
    records: Vec<String>,
    start: Option<&Json>,
    end: Option<&Json>,
    detail: Option<&Json>,
) -> Vec<String> {
    let start = history_time_arg(start);
    let end = history_time_arg(end);
    let detail = detail.and_then(|v| v.as_str());
    if start.is_none() && end.is_none() && detail.is_none() {
        return records;
    }
    records
        .into_iter()
        .filter(|r| {
            if let Some(detail) = detail {
                if !trace_detail_allows(r, detail) {
                    return false;
                }
            }
            let Some(t) = record_time_ms(r) else {
                return true;
            };
            start.map_or(true, |s| t >= s) && end.map_or(true, |e| t <= e)
        })
        .collect()
}

/// One `retrieval-history` record as JSON:
/// `[time_ms, request, [[name, text, params, activation-trace], …]]`
/// (`other-files/retrieval-history.lisp`).
fn retrieval_record(r: &crate::engine::RetrievalRecord) -> String {
    let chunks: Vec<Json> = r
        .chunks
        .iter()
        .map(|c| {
            Json::Arr(vec![
                Json::Str(c.name.clone()),
                Json::Str(c.text.clone()),
                Json::Str(c.params.clone()),
                Json::Str(c.activation_trace.clone()),
            ])
        })
        .collect();
    Json::Arr(vec![
        Json::Int(r.time_ms),
        Json::Str(r.request.clone()),
        Json::Arr(chunks),
    ])
    .write()
}

/// One `buffer-history` segment as JSON: `[time_ms, [[row, …], …]]`
/// (`other-files/buffer-history.lisp`).
fn buffer_history_record(time_ms: i64, rows: &[Vec<String>]) -> String {
    let rows: Vec<Json> = rows
        .iter()
        .map(|r| Json::Arr(r.iter().cloned().map(Json::Str).collect()))
        .collect();
    Json::Arr(vec![Json::Int(time_ms), Json::Arr(rows)]).write()
}

/// One `visicon-history` entry as JSON: `[time_ms, snapshot, …]`
/// (`other-files/perceptual-history.lisp`).
fn visicon_history_record(time_ms: i64, items: &[String]) -> String {
    let mut fields = vec![Json::Int(time_ms)];
    fields.extend(items.iter().cloned().map(Json::Str));
    Json::Arr(fields).write()
}

/// One `buffer-trace` record as JSON: `[time_ms, [summary, …]]` where each
/// summary is `[name, busy, cleared, busy->free, error, error->clear, full,
/// modified, request, chunk-name, notes]` (`tools/buffer-trace.lisp`).
fn buffer_trace_record(time_ms: i64, rows: &[crate::engine::BufSummary]) -> String {
    let flag = Json::Bool;
    let rows: Vec<Json> = rows
        .iter()
        .map(|s| {
            Json::Arr(vec![
                Json::Str(s.name.clone()),
                flag(s.busy),
                flag(s.cleared),
                flag(s.busy_free),
                flag(s.error),
                flag(s.error_clear),
                flag(s.full),
                flag(s.modified),
                Json::Str(s.request.clone().unwrap_or_default()),
                Json::Str(s.chunk_name.clone().unwrap_or_default()),
                Json::Str(s.notes.clone().unwrap_or_default()),
            ])
        })
        .collect();
    Json::Arr(vec![Json::Int(time_ms), Json::Arr(rows)]).write()
}

/// A graphic-trace rectangle being built (`gt-rect`, `env-graphic-trace.lisp`).
#[derive(Default, Clone)]
struct GtRect {
    start: i64,
    end: Option<i64>,
    request: String,
    chunk: String,
    notes: String,
}

/// Whether an optional string field is present and non-empty (`check-bs-*`).
fn nonempty(s: Option<&str>) -> bool {
    s.map(|v| !v.is_empty()).unwrap_or(false)
}

/// `parse-trace-list` (`other-files/env-graphic-trace.lisp:186`): turn the
/// `buffer-trace` records into the graphic-trace canvas items. The first item
/// is `("size" N)`; the rest are `("rectangle" …)` / `("label" …)`.
fn graphic_trace_items(
    trace: &[(i64, Vec<crate::engine::BufSummary>)],
    vertical: bool,
    now_ms: i64,
) -> Vec<String> {
    let Some(first) = trace.first() else {
        return Vec::new();
    };
    let mut buffers: Vec<String> = first.1.iter().map(|s| s.name.clone()).collect();
    if buffers.is_empty() {
        return Vec::new();
    }
    buffers.sort();
    let n = buffers.len() as i64;
    let y_inc = std::cmp::min(50, 380 / n);
    let x_inc = std::cmp::min(190, 960 / n);
    let size = 100 + trace.last().map(|(t, _)| *t).unwrap_or(0);
    let mut all = vec![Json::Arr(vec![Json::Str("size".into()), Json::Int(size)]).write()];
    let mut y = 0i64;
    let mut x_coord = 0i64;
    let mut buffer_index = 0i64;
    for x in &buffers {
        let mut rects: Vec<GtRect> = Vec::new();
        let mut current: Option<GtRect> = None;
        for (ztime, summaries) in trace {
            let Some(rec) = summaries.iter().find(|s| s.name == *x) else {
                continue;
            };
            let req = nonempty(rec.request.as_deref());
            let chunk = nonempty(rec.chunk_name.as_deref());
            let notes = nonempty(rec.notes.as_deref());
            if let Some(cur) = current.as_mut() {
                if chunk {
                    cur.chunk = rec.chunk_name.clone().unwrap_or_default();
                }
                if notes {
                    cur.notes = rec.notes.clone().unwrap_or_default();
                }
                if !rec.busy || rec.busy_free || req {
                    cur.end = Some(*ztime);
                    rects.push(current.take().unwrap());
                    if req {
                        current = Some(GtRect {
                            start: *ztime,
                            end: None,
                            request: rec.request.clone().unwrap_or_default(),
                            chunk: String::new(),
                            notes: String::new(),
                        });
                    }
                }
            } else if rec.busy {
                let rect = GtRect {
                    start: *ztime,
                    end: None,
                    request: rec.request.clone().unwrap_or_default(),
                    chunk: rec.chunk_name.clone().unwrap_or_default(),
                    notes: rec.notes.clone().unwrap_or_default(),
                };
                if req && (chunk || (rec.error && !rec.error_clear) || rec.busy_free) {
                    rects.push(GtRect {
                        end: Some(*ztime),
                        ..rect
                    });
                } else {
                    current = Some(rect);
                }
            } else if req || chunk {
                rects.push(GtRect {
                    start: *ztime,
                    end: Some(*ztime),
                    request: rec.request.clone().unwrap_or_default(),
                    chunk: rec.chunk_name.clone().unwrap_or_default(),
                    notes: rec.notes.clone().unwrap_or_default(),
                });
            }
        }
        for z in &rects {
            let end = z.end.unwrap_or(now_ms);
            let lastcol = if x == "production" {
                z.request.clone()
            } else {
                z.chunk.clone()
            };
            let parts = if !vertical {
                vec![
                    Json::Str("rectangle".into()),
                    Json::Int(z.start),
                    Json::Int(y),
                    Json::Int(end),
                    Json::Int(y + y_inc),
                    Json::Int(buffer_index),
                    Json::Str(z.request.clone()),
                    Json::Str(z.chunk.clone()),
                    Json::Str(z.notes.clone()),
                    Json::Str(lastcol),
                    Json::Str(x.clone()),
                ]
            } else {
                vec![
                    Json::Str("rectangle".into()),
                    Json::Int(x_coord),
                    Json::Int(z.start),
                    Json::Int(x_coord + x_inc),
                    Json::Int(end),
                    Json::Int(buffer_index),
                    Json::Str(z.request.clone()),
                    Json::Str(z.chunk.clone()),
                    Json::Str(z.notes.clone()),
                    Json::Str(lastcol),
                    Json::Str(x.clone()),
                ]
            };
            all.push(Json::Arr(parts).write());
        }
        let label = if !vertical {
            vec![
                Json::Str("label".into()),
                Json::Str(x.clone()),
                Json::Int(y + y_inc / 2),
                Json::Int(buffer_index),
            ]
        } else {
            vec![
                Json::Str("label".into()),
                Json::Str(x.clone()),
                Json::Int(x_coord + x_inc / 2),
                Json::Int(buffer_index),
                Json::Int(x_inc),
            ]
        };
        all.push(Json::Arr(label).write());
        y += y_inc;
        x_coord += x_inc;
        buffer_index += 1;
    }
    all
}

/// The `production-history` info summary (`get-p-history-data`'s first row):
/// `[0, "info", why-not-list, [[production, printed-text], …]]`.
fn production_info_summary(e: &crate::engine::Engine) -> Json {
    let why: Vec<Json> = e
        .production_history_why_not()
        .into_iter()
        .map(Json::Str)
        .collect();
    let prods: Vec<Json> = e
        .all_production_names()
        .into_iter()
        .map(|n| {
            let text = e.printed_production(&n).unwrap_or_default();
            Json::Arr(vec![Json::Str(n), Json::Str(text)])
        })
        .collect();
    Json::Arr(vec![
        Json::Int(0),
        Json::Str("info".into()),
        Json::Arr(why),
        Json::Arr(prods),
    ])
}

/// The recorded `production-history` records as JSON (`get-p-history-data`),
/// `[info, …records oldest-first]`.
fn production_history_records(e: &crate::engine::Engine) -> Vec<String> {
    use crate::engine::PHistTag;
    let mut out = vec![production_info_summary(e)];
    for r in &e.production_history() {
        let info = Json::Arr(r.info.iter().map(|row| Json::Arr(row.clone())).collect());
        let rec = match r.tag {
            PHistTag::Start => Json::Arr(vec![Json::Int(r.time_ms), Json::Str("start".into())]),
            PHistTag::Stop => Json::Arr(vec![Json::Int(r.time_ms), Json::Str("stop".into()), info]),
            PHistTag::Reward => Json::Arr(vec![
                Json::Int(r.time_ms),
                Json::Str("reward".into()),
                r.reward
                    .map(|v| Json::Float(v as f64))
                    .unwrap_or(Json::Null),
                info,
            ]),
            PHistTag::Conflict => Json::Arr(vec![
                Json::Int(r.time_ms),
                Json::Str("conflict-resolution".into()),
                r.selected.clone().map(Json::Str).unwrap_or(Json::Null),
                Json::Arr(r.matched.iter().cloned().map(Json::Str).collect()),
                Json::Arr(r.mismatched.iter().cloned().map(Json::Str).collect()),
                info,
            ]),
        };
        out.push(rec);
    }
    out.into_iter().map(|j| j.write()).collect()
}

/// `compute-production-grid` (`other-files/production-history.lisp:1177`): the
/// info summary followed by each conflict resolution reduced to a per-
/// production status row.
fn production_grid_records(e: &crate::engine::Engine) -> Vec<String> {
    use crate::engine::PHistTag;
    let names = e.all_production_names();
    let mut out = vec![production_info_summary(e).write()];
    for r in &e.production_history() {
        if r.tag != PHistTag::Conflict {
            continue;
        }
        let mut row: Vec<Json> = Vec::new();
        for y in &names {
            let detail = r
                .info
                .iter()
                .find(|d| d.first().and_then(|x| x.as_str()) == Some(y));
            if r.selected.as_deref() == Some(y) {
                let d = detail.cloned().unwrap_or_default();
                row.push(Json::Arr(vec![
                    Json::Int(0),
                    d.get(1).cloned().unwrap_or(Json::Null),
                    d.get(2).cloned().unwrap_or(Json::Null),
                ]));
            } else if r.matched.contains(y) {
                let d = detail.cloned().unwrap_or_default();
                row.push(Json::Arr(vec![
                    Json::Int(1),
                    d.get(1).cloned().unwrap_or(Json::Null),
                    d.get(2).cloned().unwrap_or(Json::Null),
                ]));
            } else if r.mismatched.contains(y) {
                let d = detail.cloned().unwrap_or_default();
                row.push(Json::Arr(vec![
                    Json::Int(2),
                    d.get(1).cloned().unwrap_or(Json::Null),
                ]));
            } else {
                row.push(Json::Arr(vec![Json::Int(-1)]));
            }
        }
        out.push(
            Json::Arr(vec![
                Json::Int(r.time_ms),
                Json::Str("conflict-resolution".into()),
                Json::Bool(r.selected.is_some()),
                Json::Arr(row),
            ])
            .write(),
        );
    }
    out
}

/// Parse the Lisp history envelope (`{name, date, version, comment, data}`),
/// accepting the old bare-array form too.
fn parse_saved_history(text: &str) -> Option<(Option<String>, Vec<String>)> {
    let v = crate::json::parse(text).ok()?;
    match v {
        Json::Arr(records) => Some((None, records.iter().map(Json::write).collect())),
        Json::Obj(_) => {
            let data = v.get("data")?;
            let Json::Arr(records) = data else {
                return None;
            };
            let comment = v
                .get("comment")
                .and_then(|c| c.as_str())
                .map(str::to_string);
            Some((comment, records.iter().map(Json::write).collect()))
        }
        _ => None,
    }
}

fn blending_trace_records(records: &[crate::engine::BlendingRecord]) -> Vec<String> {
    records
        .iter()
        .map(|r| {
            Json::Arr(vec![
                Json::Int(r.time_ms),
                Json::Str(r.request.clone()),
                Json::Str(r.result.clone()),
                Json::Str(r.trace.clone()),
            ])
            .write()
        })
        .collect()
}

fn blending_environment_records(e: &crate::engine::Engine) -> Vec<String> {
    e.blending_history()
        .iter()
        .map(|r| {
            let chunks: Vec<Json> = r
                .chunks
                .iter()
                .map(|(name, act, p)| {
                    Json::Arr(vec![
                        Json::Str(name.clone()),
                        Json::Float(*act as f64),
                        Json::Float(*p as f64),
                        Json::Str(e.printed_chunk(name).unwrap_or_default()),
                    ])
                })
                .collect();
            Json::Arr(vec![
                Json::Int(r.time_ms),
                Json::Str(r.request.clone()),
                Json::Str(r.result.clone()),
                Json::Arr(chunks),
                Json::Str(r.trace.clone()),
            ])
            .write()
        })
        .collect()
}

/// Group record JSON strings into array chunks of ~`size` bytes (each chunk a
/// valid array, so the client can decode it independently).
fn chunk_records(records: &[String], size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    let mut cur_len = 2usize; // `[]`
    for rec in records {
        if !cur.is_empty() && cur_len + rec.len() + 1 > size {
            chunks.push(format!("[{}]", cur.join(",")));
            cur.clear();
            cur_len = 2;
        }
        cur_len += rec.len() + 1;
        cur.push(rec);
    }
    if !cur.is_empty() {
        chunks.push(format!("[{}]", cur.join(",")));
    }
    chunks
}

/// A `Vec<String>` (or `None`) as JSON string values.
fn strings(v: Option<Vec<String>>) -> Vec<Json> {
    v.unwrap_or_default().into_iter().map(Json::Str).collect()
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

/// Coerce a JSON list (or scalar) into `f64`s for the stats commands. Nested
/// matrices (the Siegler response tables) flatten row-major, matching
/// `numbers-list` (`act-gui-interface.lisp`).
fn num_list(j: Option<&Json>) -> Vec<f64> {
    fn collect(j: &Json, out: &mut Vec<f64>) {
        match j {
            Json::Arr(a) => {
                for x in a {
                    collect(x, out);
                }
            }
            Json::Int(i) => out.push(*i as f64),
            Json::Float(x) => out.push(*x),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Some(j) = j {
        collect(j, &mut out);
    }
    out
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

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("models")
    }

    fn fixture_model(name: &str) -> String {
        fixture_root()
            .join(name.replace(';', std::path::MAIN_SEPARATOR_STR))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn logical_model_paths_are_rejected() {
        assert!(filesystem_path("ACT-R:tutorial;unit1;count.lisp").is_err());
        assert!(filesystem_path("tutorial;unit1;count.lisp").is_err());
        assert_eq!(
            filesystem_path("models/count.lisp").unwrap(),
            PathBuf::from("models/count.lisp")
        );
        assert_eq!(
            filesystem_path("C:\\models\\count.lisp").unwrap(),
            PathBuf::from("C:\\models\\count.lisp")
        );
    }

    #[test]
    fn start_pipe_connection_declines_socket_fallback() {
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        let v = s
            .evaluate("start-pipe-connection", None, &[], &mut remote)
            .unwrap();
        assert_eq!(v, vec![Json::Bool(false)]);
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
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
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
    fn semantic_load_warnings_are_streamed() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("warning-trace", "sink", MonitorWhen::Simple);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;semantic.lisp"))],
            &mut remote,
        )
        .unwrap();
        let trace = s.take_trace_events();
        for name in ["CATEGORY", "PENDING", "YES", "NO"] {
            let warning = format!("Creating chunk {name} with no slots");
            assert!(
                trace
                    .iter()
                    .any(|(line, kind)| *kind == TraceKind::Warning && line.contains(&warning)),
                "{trace:?}"
            );
        }
    }

    #[test]
    fn model_v_nil_suppresses_run_trace() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model(
                "tutorial;unit4;zbrodoff-model.lisp",
            ))],
            &mut remote,
        )
        .unwrap();
        assert!(s.take_trace().is_empty());
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        assert!(s.take_trace().is_empty());
    }

    #[test]
    fn malformed_model_file_is_rejected() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        let err = s
            .evaluate(
                "load-act-r-model",
                None,
                &[Json::Str(fixture_model(
                    "tutorial;unit1;broken-addition.lisp",
                ))],
                &mut remote,
            )
            .unwrap_err();
        assert!(err.contains("parse error"), "{err}");
    }

    #[test]
    fn invalid_move_cursor_location_warns() {
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("warning-trace", "sink", MonitorWhen::Simple);
        s.load_source(
            "(define-model move-test \
             (chunk-type goal state) \
             (define-chunks (start-pos screen-x 100 screen-y 0)) \
             (p start ?goal> buffer empty ==> +goal> +manual> isa move-cursor loc start-pos))",
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            trace
                .iter()
                .any(|line| line
                    .contains("No valid location could be generated from NIL or START-POS")),
            "{trace:?}"
        );
    }

    #[test]
    fn model_body_output_is_streamed() {
        let mut s = Session::new((100, 0));
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("model-trace", "sink", MonitorWhen::Simple);
        s.load_source("(define-model output-test (sgp :v t) (model-output \"hello\"))")
            .unwrap();
        let trace = s.take_trace();
        assert!(trace.iter().any(|line| line == "hello"), "{trace:?}");
    }

    #[test]
    fn invalid_scheduled_commands_are_skipped_silently() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("warning-trace", "sink", MonitorWhen::Simple);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model(
                "tutorial;unit7;utility-learning-issues-model.lisp",
            ))],
            &mut remote,
        )
        .unwrap();
        let load = s.take_trace();
        assert!(
            load.iter()
                .any(|line| line.contains("Can't schedule \"utility-learning-issues-choose\"")),
            "{load:?}"
        );
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            !trace
                .iter()
                .any(|line| line.contains("utility-learning-issues-choose")),
            "{trace:?}"
        );
    }

    fn stepper_fixture() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("stepper")
            .join("stepper-model.lisp")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn stepper_pauses_before_and_updates_after_an_event() {
        let model = stepper_fixture();
        if !PathBuf::from(&model).is_file() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("next-event", Some("next-event".into()), None);
        s.add_remote("update-event", Some("update-event".into()), None);
        s.evaluate("load-act-r-model", None, &[Json::Str(model)], &mut remote)
            .unwrap();
        let started = s
            .evaluate(
                "start-stepper",
                None,
                &[
                    Json::Bool(false),
                    Json::Bool(false),
                    Json::Str("next-event".into()),
                    Json::Str("update-event".into()),
                ],
                &mut remote,
            )
            .unwrap();
        assert_eq!(started[0], Json::Bool(true));
        let id = started[1].as_str().unwrap().to_string();
        s.evaluate(
            "set-stepper-ready",
            None,
            &[Json::Str(id.clone())],
            &mut remote,
        )
        .unwrap();

        let (limit, full, should_run) = s.run_begin("run", 10.0);
        assert!(should_run);
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperPause
        );
        let (cmd, _model, args) = s.take_stepper_pause().unwrap();
        assert_eq!(cmd, "next-event");
        assert!(args[0].as_str().unwrap().contains("SET-BUFFER-CHUNK GOAL"));

        s.evaluate("step-stepper", None, &[Json::Str(id.clone())], &mut remote)
            .unwrap();
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperUpdate
        );
        let (cmd, _model, args) = s.take_stepper_update().unwrap();
        assert_eq!(cmd, "update-event");
        assert_eq!(args.len(), 7);
        assert!(args[0].as_str().unwrap().contains("SET-BUFFER-CHUNK GOAL"));

        s.evaluate("stop-stepper", None, &[Json::Str(id)], &mut remote)
            .unwrap();
    }

    #[test]
    fn multi_model_stepper_reports_the_paused_model() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("next-event", Some("next-event".into()), None);
        s.add_remote("update-event", Some("update-event".into()), None);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut remote,
        )
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
            &mut remote,
        )
        .unwrap();
        let started = s
            .evaluate(
                "start-stepper",
                None,
                &[
                    Json::Bool(false),
                    Json::Bool(false),
                    Json::Str("next-event".into()),
                    Json::Str("update-event".into()),
                ],
                &mut remote,
            )
            .unwrap();
        let id = started[1].as_str().unwrap().to_string();
        s.evaluate(
            "set-stepper-ready",
            None,
            &[Json::Str(id.clone())],
            &mut remote,
        )
        .unwrap();
        let (limit, full, should_run) = s.run_begin("run", 1.0);
        assert!(should_run);
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperPause
        );
        let (_, model, _) = s.take_stepper_pause().unwrap();
        assert!(model.is_some());
        s.evaluate("stop-stepper", None, &[Json::Str(id)], &mut remote)
            .unwrap();
    }

    #[test]
    fn stepper_tutoring_conflict_payload() {
        let model = stepper_fixture();
        if !PathBuf::from(&model).is_file() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("next-event", Some("next-event".into()), None);
        s.add_remote("update-event", Some("update-event".into()), None);
        s.evaluate("load-act-r-model", None, &[Json::Str(model)], &mut remote)
            .unwrap();
        let started = s
            .evaluate(
                "start-stepper",
                None,
                &[
                    Json::Bool(false),
                    Json::Bool(true),
                    Json::Str("next-event".into()),
                    Json::Str("update-event".into()),
                ],
                &mut remote,
            )
            .unwrap();
        let id = started[1].as_str().unwrap().to_string();
        s.evaluate(
            "set-stepper-ready",
            None,
            &[Json::Str(id.clone())],
            &mut remote,
        )
        .unwrap();
        let (limit, full, _) = s.run_begin("run", 10.0);
        let mut found = false;
        for _ in 0..40 {
            match s.run_slice_reason(limit, full).unwrap() {
                RunStop::StepperPause => {
                    let _ = s.take_stepper_pause();
                    s.evaluate("step-stepper", None, &[Json::Str(id.clone())], &mut remote)
                        .unwrap();
                }
                RunStop::StepperUpdate => {
                    let (_, _model, args) = s.take_stepper_update().unwrap();
                    if args.get(2).and_then(|v| v.as_str()) == Some("conflict-resolution") {
                        let data = s
                            .evaluate("update-stepper", None, &[Json::Bool(false)], &mut remote)
                            .unwrap();
                        assert!(matches!(data.first(), Some(Json::Arr(rows)) if !rows.is_empty()));
                        found = true;
                        break;
                    }
                }
                RunStop::DeviceAction | RunStop::Continue => continue,
                RunStop::Finished => break,
            }
        }
        assert!(found, "never reached conflict-resolution tutoring");
        s.evaluate("stop-stepper", None, &[Json::Str(id)], &mut remote)
            .unwrap();
    }

    fn started_stepper(step_all: bool) -> (Session, String) {
        let model = stepper_fixture();
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.add_remote("next-event", Some("next-event".into()), None);
        s.add_remote("update-event", Some("update-event".into()), None);
        s.evaluate("load-act-r-model", None, &[Json::Str(model)], &mut remote)
            .unwrap();
        let started = s
            .evaluate(
                "start-stepper",
                None,
                &[
                    Json::Bool(step_all),
                    Json::Bool(false),
                    Json::Str("next-event".into()),
                    Json::Str("update-event".into()),
                ],
                &mut remote,
            )
            .unwrap();
        let id = started[1].as_str().unwrap().to_string();
        s.evaluate(
            "set-stepper-ready",
            None,
            &[Json::Str(id.clone())],
            &mut remote,
        )
        .unwrap();
        (s, id)
    }

    #[test]
    fn stepper_production_constraint_and_update_details() {
        let model = stepper_fixture();
        if !PathBuf::from(&model).is_file() {
            return;
        }
        let (mut s, id) = started_stepper(false);
        let mut remote = NoRemote;
        let (limit, full, _) = s.run_begin("run", 10.0);
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperPause
        );
        let _ = s.take_stepper_pause().unwrap();
        let stepped = s
            .evaluate(
                "step-stepper",
                None,
                &[
                    Json::Str(id.clone()),
                    Json::Bool(false),
                    Json::Str("production".into()),
                    Json::Str("GOT-HIT".into()),
                ],
                &mut remote,
            )
            .unwrap();
        assert_eq!(stepped[0], Json::Bool(true));
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperUpdate
        );
        let _ = s.take_stepper_update().unwrap();
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperPause
        );
        let (_, _model, args) = s.take_stepper_pause().unwrap();
        assert!(args[0]
            .as_str()
            .unwrap()
            .contains("PRODUCTION-SELECTED GOT-HIT"));

        s.evaluate("step-stepper", None, &[Json::Str(id.clone())], &mut remote)
            .unwrap();
        assert_eq!(
            s.run_slice_reason(limit, full).unwrap(),
            RunStop::StepperUpdate
        );
        let (_, _model, args) = s.take_stepper_update().unwrap();
        assert_eq!(args[2], Json::Str("production-selected".into()));
        let detail = s
            .evaluate(
                "update-stepper",
                None,
                &[Json::Str("GOT-HIT".into())],
                &mut remote,
            )
            .unwrap();
        assert_eq!(detail.len(), 4);
        assert!(detail[0]
            .as_str()
            .unwrap()
            .contains("Parameters for production GOT-HIT"));
        assert!(detail[1].as_str().unwrap().contains("VALUE: ONE"));
        assert!(detail[2].as_str().unwrap().contains("(P GOT-HIT"));
        assert!(matches!(detail[3], Json::Arr(_)));
        s.evaluate("stop-stepper", None, &[Json::Str(id)], &mut remote)
            .unwrap();
    }

    /// A remote that records callbacks, for monitor registration tests.
    #[derive(Default)]
    struct ModelMonitor {
        calls: Vec<(String, Option<String>, Vec<Json>)>,
    }
    impl Remote for ModelMonitor {
        fn call(
            &mut self,
            name: &str,
            model: Option<&str>,
            args: Vec<Json>,
        ) -> Result<Vec<Json>, String> {
            self.calls
                .push((name.to_string(), model.map(str::to_string), args));
            Ok(vec![Json::Bool(true)])
        }
    }

    #[test]
    fn creating_model_monitor_and_mp_models_report_loaded_model() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = ModelMonitor::default();
        s.add_remote("env-current-add", Some("env-current-add".into()), None);
        s.add_monitor("creating-model", "env-current-add", MonitorWhen::Simple);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut remote,
        )
        .unwrap();
        assert_eq!(
            remote.calls,
            vec![(
                "env-current-add".to_string(),
                Some("COUNT".to_string()),
                vec![Json::Str("COUNT".into())],
            )]
        );
        assert_eq!(
            s.evaluate("mp-models", None, &[], &mut remote).unwrap(),
            vec![Json::Arr(vec![Json::Str("COUNT".into())])]
        );
    }

    #[test]
    fn environment_sessions_load_and_switch_models() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
            &mut r,
        )
        .unwrap();
        assert_eq!(
            s.evaluate("mp-models", None, &[], &mut r).unwrap(),
            vec![Json::Arr(vec![
                Json::Str("COUNT".into()),
                Json::Str("ADDITION".into())
            ])]
        );
        assert_eq!(
            s.evaluate("current-model", None, &[], &mut r).unwrap(),
            vec![Json::Str("ADDITION".into())]
        );
        s.evaluate(
            "set-current-model",
            None,
            &[Json::Str("COUNT".into())],
            &mut r,
        )
        .unwrap();
        assert_eq!(
            s.evaluate("current-model", None, &[], &mut r).unwrap(),
            vec![Json::Str("COUNT".into())]
        );
    }

    #[test]
    fn retrieval_history_is_per_model() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            Some("COUNT"),
            &[Json::Str("retrieval-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            Some("ADDITION"),
            &[Json::Str("retrieval-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        s.evaluate(
            "set-current-model",
            None,
            &[Json::Str("COUNT".into())],
            &mut r,
        )
        .unwrap();
        let count = s.history_records("RETRIEVAL-HISTORY").unwrap();
        assert!(!count.is_empty(), "COUNT produced no retrieval history");
        s.evaluate(
            "set-current-model",
            None,
            &[Json::Str("ADDITION".into())],
            &mut r,
        )
        .unwrap();
        let addition = s.history_records("RETRIEVAL-HISTORY").unwrap();
        assert!(
            !addition.is_empty(),
            "ADDITION produced no retrieval history"
        );
        assert_ne!(count, addition, "model histories were mixed");
    }

    /// A client-command handler that records the display ops it receives
    /// (`visible-virtual-window` protocol) and accepts every `check`.
    #[derive(Default)]
    struct VvRemote {
        ops: Vec<Vec<Json>>,
    }
    impl Remote for VvRemote {
        fn call(
            &mut self,
            _name: &str,
            _model: Option<&str>,
            args: Vec<Json>,
        ) -> Result<Vec<Json>, String> {
            self.ops.push(args);
            Ok(vec![Json::Bool(true)])
        }
    }

    /// The visible-virtual window protocol: `check` on open, then the item
    /// ops, dispatched to the client handler.
    #[test]
    fn visible_window_protocol() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = VvRemote::default();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        // Register the handler command, then the virtual-window handler.
        s.add_remote("env-vv", Some("env-vv".into()), None);
        s.evaluate(
            "add-virtual-window-handler",
            None,
            &[Json::Str("env-vv".into())],
            &mut r,
        )
        .unwrap();
        assert_eq!(
            s.evaluate("visible-virtuals-available?", None, &[], &mut r)
                .unwrap()[0],
            Json::Bool(true)
        );

        let verb = |o: &Vec<Json>| o.first().and_then(|x| x.as_str()).unwrap_or("").to_string();
        s.evaluate(
            "open-exp-window",
            None,
            &[
                Json::Str("Test".into()),
                Json::Obj(vec![
                    ("x".into(), Json::Int(10)),
                    ("y".into(), Json::Int(20)),
                    ("width".into(), Json::Int(100)),
                    ("height".into(), Json::Int(80)),
                ]),
            ],
            &mut r,
        )
        .unwrap();
        assert!(r.ops.iter().any(|o| verb(o) == "check"));
        assert!(r.ops.iter().any(|o| verb(o) == "open"));

        s.evaluate(
            "add-text-to-exp-window",
            None,
            &[
                Json::Str("hi".into()),
                Json::Obj(vec![("x".into(), Json::Int(5)), ("y".into(), Json::Int(6))]),
            ],
            &mut r,
        )
        .unwrap();
        let text = r.ops.iter().find(|o| verb(o) == "text").expect("text op");
        assert!(text[2]
            .as_str()
            .map(|s| s.starts_with("TEXT-VDI"))
            .unwrap_or(false));
    }

    /// `record-history "trace-history"` captures the trace and the
    /// incremental protocol streams it back in JSON chunks.
    #[test]
    fn trace_history_incremental() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace(); // drain the run's trace into the history

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("trace-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("trace-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        assert!(matches!(id, Json::Int(_)), "expected an id, got {id:?}");

        let mut chunks = 0;
        loop {
            let got = s
                .evaluate("get-incremental-history-data", None, &[id.clone()], &mut r)
                .unwrap();
            match got.first().and_then(|x| x.as_str()) {
                Some("") | None => break,
                Some(c) => {
                    assert!(c.starts_with('['), "chunk not JSON array: {c}");
                    chunks += 1;
                }
            }
            assert!(chunks < 1000, "incremental stream did not terminate");
        }
        assert!(chunks >= 1, "expected at least one chunk");
    }

    #[test]
    fn history_start_and_end_filters() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace();

        let none = s
            .evaluate(
                "history-data-available",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Int(100),
                    Json::Int(200),
                ],
                &mut r,
            )
            .unwrap();
        assert_eq!(none[0], Json::Bool(false));

        let some = s
            .evaluate(
                "history-data-available",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Float(0.0),
                    Json::Float(1.0),
                ],
                &mut r,
            )
            .unwrap();
        assert_eq!(some[0], Json::Bool(true));

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Int(16000),
                    Json::Null,
                    Json::Float(0.0),
                    Json::Float(0.2),
                ],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(records) = crate::json::parse(chunk[0].as_str().unwrap()).unwrap() else {
            panic!("filtered trace is not an array");
        };
        assert!(!records.is_empty());
        for rec in records {
            let Json::Arr(fields) = rec else {
                panic!("trace record is not an array");
            };
            let t = fields[0].as_f64().unwrap();
            assert!((0.0..=200.0).contains(&t), "record outside range: {t}");
        }
    }

    /// `record-history "retrieval-history"` records DM retrievals in the
    /// Environment's record shape.
    #[test]
    fn retrieval_history_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("retrieval-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("retrieval-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true), "count.lisp retrieves");

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("retrieval-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let text = chunk[0].as_str().unwrap();
        let Json::Arr(recs) = crate::json::parse(text).unwrap() else {
            panic!("history payload is not a JSON array");
        };
        assert!(!recs.is_empty(), "expected at least one retrieval record");
        let Json::Arr(fields) = &recs[0] else {
            panic!("record is not an array");
        };
        assert!(matches!(fields[0], Json::Int(_)), "record[0] = time");
        assert!(matches!(fields[1], Json::Str(_)), "record[1] = request");
        let Json::Arr(chunks) = &fields[2] else {
            panic!("record[2] is not the chunk set");
        };
        // Each chunk: [name, text, params, activation-trace].
        let Json::Arr(ci) = &chunks[0] else {
            panic!("chunk record is not an array");
        };
        assert_eq!(ci.len(), 4);
        // A real candidate's params carry the sdp activation block.
        let real = chunks.iter().find_map(|c| match c {
            Json::Arr(f) if f.first().and_then(|x| x.as_str()) != Some(":retrieval-failure") => {
                Some(f)
            }
            _ => None,
        });
        if let Some(f) = real {
            let params = f[2].as_str().unwrap_or("");
            assert!(
                params.contains(":Activation") && params.contains(":Base-Level"),
                "params sdp block: {params:?}"
            );
        }
    }

    /// `record-history "buffer-history"` records buffer changes in the
    /// Environment's segment/row shape.
    #[test]
    fn buffer_history_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("buffer-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("buffer-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("buffer-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let text = chunk[0].as_str().unwrap();
        let Json::Arr(segs) = crate::json::parse(text).unwrap() else {
            panic!("buffer-history payload is not a JSON array");
        };
        assert!(!segs.is_empty(), "expected at least one segment");
        let Json::Arr(seg) = &segs[0] else {
            panic!("segment is not an array");
        };
        assert!(matches!(seg[0], Json::Int(_)), "segment[0] = time");
        let Json::Arr(rows) = &seg[1] else {
            panic!("segment[1] is not the row list");
        };
        assert!(!rows.is_empty(), "expected at least one buffer row");
        let Json::Arr(row) = &rows[0] else {
            panic!("row is not an array");
        };
        assert_eq!(row.len(), 8, "row has 8 fields");
    }

    /// `record-history "visicon-history"` records the printed visicon on each
    /// update.
    #[test]
    fn visicon_history_records() {
        use crate::engine::{DeviceSetup, DeviceText};
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.set_device(DeviceSetup {
            texts: vec![DeviceText {
                at_ms: 0,
                text: "A".to_string(),
                x: 100,
                y: 100,
                run: None,
                seq: 0,
            }],
            ..Default::default()
        })
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("visicon-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("visicon-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("visicon-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let text = chunk[0].as_str().unwrap();
        let Json::Arr(entries) = crate::json::parse(text).unwrap() else {
            panic!("visicon-history payload is not a JSON array");
        };
        assert!(!entries.is_empty());
        let Json::Arr(entry) = &entries[0] else {
            panic!("entry is not an array");
        };
        assert!(matches!(entry[0], Json::Int(_)), "entry[0] = time");
        assert!(entry.len() >= 2, "entry has at least one snapshot");
    }

    #[test]
    fn audicon_history_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("audicon-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "new-tone-sound",
            None,
            &[
                Json::Int(1000),
                Json::Float(0.1),
                Json::Float(0.0),
                Json::Bool(true),
            ],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(1)], &mut r).unwrap();
        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("audicon-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));
        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("audicon-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(entries) = crate::json::parse(chunk[0].as_str().unwrap()).unwrap() else {
            panic!("audicon-history payload is not an array");
        };
        assert!(!entries.is_empty());
    }

    #[test]
    fn blending_history_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((1, 1));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model(
                "extras;blending;blending-test-7.lisp",
            ))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("blending-trace".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(5)], &mut r).unwrap();
        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("blending-trace".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));
        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[
                    Json::Str("blending-environment-history".into()),
                    Json::Int(16000),
                ],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(records) = crate::json::parse(chunk[0].as_str().unwrap()).unwrap() else {
            panic!("blending history payload is not an array");
        };
        let Json::Arr(first) = &records[0] else {
            panic!("blending record is not an array");
        };
        assert_eq!(first.len(), 5);
        assert!(first[1].as_str().is_some_and(|s| !s.is_empty()));
        assert!(first[2].as_str().is_some_and(|s| !s.is_empty()));
        let Json::Arr(chunks) = &first[3] else {
            panic!("blending chunks are not an array");
        };
        assert!(!chunks.is_empty());
        let trace = first[4].as_str().expect("blending trace");
        assert!(trace.contains("Finding blended value for slot"), "{trace}");
        assert!(trace.contains(" Final result:"), "{trace}");
        assert!(
            trace.contains("This is the definition of the blended chunk"),
            "{trace}"
        );
    }

    #[test]
    fn history_save_load_uses_the_lisp_envelope() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace();
        let path = std::env::temp_dir().join(format!(
            "actr-history-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = path.to_string_lossy().into_owned();
        let saved = s
            .evaluate(
                "save-history-data",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Str(file.clone()),
                    Json::Str("round trip".into()),
                ],
                &mut r,
            )
            .unwrap();
        assert_eq!(saved[0], Json::Bool(true));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"comment\":\"round trip\""), "{text}");
        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Int(16000),
                    Json::Str(file),
                ],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let comment = s
            .evaluate("get-incremental-history-data", None, &[id.clone()], &mut r)
            .unwrap();
        assert_eq!(comment[0].as_str(), Some("\"round trip\""));
        let data = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(records) = crate::json::parse(data[0].as_str().unwrap()).unwrap() else {
            panic!("saved trace data is not an array");
        };
        assert!(!records.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn trace_history_save_filters_detail_and_time() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace();
        let path = std::env::temp_dir().join(format!(
            "actr-trace-filter-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = path.to_string_lossy().into_owned();
        let saved = s
            .evaluate(
                "save-history-data",
                None,
                &[
                    Json::Str("trace-history".into()),
                    Json::Str(file),
                    Json::Str("filtered".into()),
                    Json::Float(0.0),
                    Json::Float(10.0),
                    Json::Str("low".into()),
                ],
                &mut r,
            )
            .unwrap();
        assert_eq!(saved[0], Json::Bool(true));
        let Json::Obj(envelope) =
            crate::json::parse(&std::fs::read_to_string(&path).unwrap()).unwrap()
        else {
            panic!("saved history is not an object");
        };
        let data = envelope
            .iter()
            .find(|(k, _)| k == "data")
            .map(|(_, v)| v)
            .expect("history data");
        let Json::Arr(records) = data else {
            panic!("history data is not an array");
        };
        assert!(!records.is_empty());
        for rec in records {
            let Json::Arr(fields) = rec else {
                panic!("trace record is not an array");
            };
            assert_eq!(fields[1].as_str(), Some("low"));
        }
        let _ = std::fs::remove_file(path);
    }

    /// `record-history "buffer-trace"` + a buffer name records the buffer
    /// timeline in the Environment's summary shape.
    #[test]
    fn buffer_trace_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("buffer-trace".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("record-history", None, &[Json::Str("goal".into())], &mut r)
            .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("buffer-trace".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("buffer-trace".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let text = chunk[0].as_str().unwrap();
        let Json::Arr(records) = crate::json::parse(text).unwrap() else {
            panic!("buffer-trace payload is not a JSON array");
        };
        assert!(!records.is_empty());
        let Json::Arr(rec) = &records[0] else {
            panic!("record is not an array");
        };
        assert!(matches!(rec[0], Json::Int(_)), "record[0] = time");
        let Json::Arr(rows) = &rec[1] else {
            panic!("record[1] is not the summary list");
        };
        let Json::Arr(row) = &rows[0] else {
            panic!("summary is not an array");
        };
        assert_eq!(row.len(), 11, "summary has 11 fields");
        assert_eq!(row[0].as_str(), Some("GOAL"));
    }

    /// The graphic traces are processors over `buffer-trace`.
    #[test]
    fn graphic_trace_processes_buffer_trace() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("buffer-trace".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("record-history", None, &[Json::Str("goal".into())], &mut r)
            .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[
                    Json::Str("horizontal-graphic-trace".into()),
                    Json::Int(16000),
                ],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        assert!(matches!(id, Json::Int(_)), "expected an id, got {id:?}");
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let text = chunk[0].as_str().unwrap();
        let Json::Arr(items) = crate::json::parse(text).unwrap() else {
            panic!("graphic-trace payload is not a JSON array");
        };
        assert!(!items.is_empty());
        let Json::Arr(first) = &items[0] else {
            panic!("item is not an array");
        };
        assert_eq!(first[0].as_str(), Some("size"));
        assert!(items.iter().any(|i| matches!(i, Json::Arr(f)
            if matches!(f[0].as_str(), Some("label") | Some("rectangle")))));
    }

    /// `record-history "production-history"` records conflict resolutions, and
    /// the `production-grid` processor reduces them per production.
    #[test]
    fn production_history_records() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("production-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let fetch = |s: &mut Session, r: &mut NoRemote, name: &str| -> Json {
            let id = s
                .evaluate(
                    "start-incremental-history-data",
                    None,
                    &[Json::Str(name.into()), Json::Int(16000)],
                    r,
                )
                .unwrap()[0]
                .clone();
            let chunk = s
                .evaluate("get-incremental-history-data", None, &[id], r)
                .unwrap();
            crate::json::parse(chunk[0].as_str().unwrap()).unwrap()
        };

        let avail = s
            .evaluate(
                "history-data-available",
                None,
                &[Json::Str("production-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(avail[0], Json::Bool(true));

        let Json::Arr(recs) = fetch(&mut s, &mut r, "production-history") else {
            panic!("production-history payload is not a JSON array");
        };
        // The info summary is first.
        let Json::Arr(info) = &recs[0] else {
            panic!("first record is not an array");
        };
        assert_eq!(info[1].as_str(), Some("info"));
        // A conflict-resolution record with the info field.
        let conflict = recs
            .iter()
            .find(|x| matches!(x, Json::Arr(f) if f.get(1).and_then(|y| y.as_str()) == Some("conflict-resolution")));
        assert!(conflict.is_some(), "expected a conflict-resolution record");

        // production-grid: info summary then a transformed conflict record.
        let Json::Arr(grid) = fetch(&mut s, &mut r, "production-grid") else {
            panic!("production-grid payload is not an JSON array");
        };
        assert!(matches!(grid.first(), Some(Json::Arr(f)) if f[1].as_str() == Some("info")));
        assert!(grid.iter().any(|x| matches!(x, Json::Arr(f)
            if f.get(1).and_then(|y| y.as_str()) == Some("conflict-resolution"))));
    }

    /// `step-once` executes one event without a stop terminator.
    #[test]
    fn single_step_executes_one_event() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        let mut steps = 0;
        for _ in 0..5 {
            let b = s.evaluate("step-once", None, &[], &mut r).unwrap();
            if b[0] == Json::Bool(false) {
                break;
            }
            steps += 1;
        }
        assert!(steps >= 1, "expected to step at least one event");
        let trace = s.take_trace();
        assert!(!trace.is_empty(), "stepping should produce trace");
        assert!(
            !trace.iter().any(|l| l.contains("Stopped because")),
            "single-step must not emit a stop terminator"
        );
    }

    /// The BOLD predictions are processors over `buffer-trace`.
    #[test]
    fn bold_prediction_processes_buffer_trace() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("buffer-trace".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("record-history", None, &[Json::Str("goal".into())], &mut r)
            .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[
                    Json::Str("bold-prediction-with-time".into()),
                    Json::Int(16000),
                ],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        assert!(matches!(id, Json::Int(_)), "expected an id, got {id:?}");
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(items) = crate::json::parse(chunk[0].as_str().unwrap()).unwrap() else {
            panic!("bold payload is not a JSON array");
        };
        assert!(items.len() >= 2, "expected a time header + a buffer row");
        let Json::Arr(header) = &items[0] else {
            panic!("time header is not an array");
        };
        assert_eq!(header[0].as_str(), Some("time"));
        for row in &items[1..] {
            let Json::Arr(row) = row else {
                panic!("BOLD buffer row is not an array");
            };
            assert!(row[0].as_str().is_some_and(|s| !s.is_empty()));
            assert_eq!(row.len(), header.len());
            assert!(row[1..].iter().all(|v| v.as_f64().is_some()));
        }
    }

    /// `production-graph` returns `[nodes, edges]` from production-history.
    #[test]
    fn production_graph_command() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("production-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let v = s.evaluate("production-graph", None, &[], &mut r).unwrap();
        let Json::Arr(parts) = &v[0] else {
            panic!("production-graph result is not an array");
        };
        let Json::Arr(nodes) = &parts[0] else {
            panic!("nodes are not an array");
        };
        assert!(!nodes.is_empty());
        assert!(matches!(parts.get(1), Some(Json::Arr(_))));
        for command in [
            "production-graph-all",
            "production-graph-freq",
            "production-graph-cycles",
            "production-graph-u-cycles",
            "production-graph-runs",
            "production-graph-u-runs",
            "production-graph-utility",
        ] {
            let v = s.evaluate(command, None, &[], &mut r).unwrap();
            let data = &v[0];
            assert!(data.get("nodes").is_some(), "{command}");
            assert!(data.get("edges").is_some(), "{command}");
            assert!(data.get("pages").is_some(), "{command}");
        }
    }

    /// The Environment viewer commands return the model's productions,
    /// buffers and DM chunks.
    #[test]
    fn environment_viewer_commands() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut remote,
        )
        .unwrap();

        fn names(v: &[Json]) -> Vec<String> {
            match v.first() {
                Some(Json::Arr(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            }
        }

        let version = s.evaluate("act-r-version", None, &[], &mut remote).unwrap();
        assert!(matches!(version.first(), Some(Json::Str(_))));

        let prods = s
            .evaluate("all-productions", None, &[], &mut remote)
            .unwrap();
        assert!(!names(&prods).is_empty(), "expected productions");

        let buffers = s.evaluate("buffers", None, &[], &mut remote).unwrap();
        assert!(names(&buffers).iter().any(|b| b == "GOAL"));

        let chunks = s.evaluate("chunks", None, &[], &mut remote).unwrap();
        assert!(!names(&chunks).is_empty(), "count.lisp has add-dm chunks");

        // `_none_` (no filter) returns every DM chunk, in the same order.
        let all = s
            .evaluate(
                "filter-dm-chunks",
                None,
                &[Json::Str("_none_".into())],
                &mut remote,
            )
            .unwrap();
        assert_eq!(names(&all), names(&chunks));

        // A slot filter the model cannot satisfy returns nothing.
        let none = s
            .evaluate(
                "filter-dm-chunks",
                None,
                &[Json::Str("NO-SUCH-SLOT".into())],
                &mut remote,
            )
            .unwrap();
        assert!(names(&none).is_empty());

        // Parameter viewer: modules, their parameters, and a value read-out.
        let modules = s
            .evaluate("modules-with-parameters", None, &[], &mut remote)
            .unwrap();
        assert!(names(&modules).iter().any(|m| m == "declarative"));
        let params = s
            .evaluate(
                "modules-parameters",
                None,
                &[Json::Str("declarative".into())],
                &mut remote,
            )
            .unwrap();
        assert!(names(&params).iter().any(|p| p == ":RT"));
        let detail = s
            .evaluate(
                "printed-parameter-details",
                None,
                &[Json::Str(":RT".into())],
                &mut remote,
            )
            .unwrap();
        assert!(matches!(detail.first(), Some(Json::Str(s)) if s.starts_with(":RT ")));

        let modules = s
            .evaluate("sorted-module-names", None, &[], &mut remote)
            .unwrap();
        assert!(
            names(&modules).iter().any(|m| m == "BLENDING"),
            "blending module missing: {modules:?}"
        );

        let chunk = names(&chunks).first().cloned().unwrap_or_default();
        let dm_detail = s
            .evaluate(
                "dm-chunk-details",
                None,
                &[Json::Str(chunk.clone())],
                &mut remote,
            )
            .unwrap();
        assert!(
            matches!(dm_detail.first(), Some(Json::Str(s)) if s.contains("Declarative parameters") && s.contains(&chunk)),
            "DM detail omitted sdp or chunk: {dm_detail:?}"
        );

        let prod = names(&prods).first().cloned().unwrap_or_default();
        let prod_detail = s
            .evaluate(
                "production-details",
                None,
                &[Json::Str(prod.clone())],
                &mut remote,
            )
            .unwrap();
        assert!(
            matches!(prod_detail.first(), Some(Json::Str(s)) if s.contains("Parameters for production") && s.contains(&prod)),
            "production detail omitted spp or text: {prod_detail:?}"
        );
    }

    #[test]
    fn runtime_define_chunks_does_not_replay_multi_model_trace() {
        if !fixture_root().is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        for model in ["tutorial;unit1;count.lisp", "tutorial;unit1;semantic.lisp"] {
            s.evaluate(
                "load-act-r-model",
                None,
                &[Json::Str(fixture_model(model))],
                &mut remote,
            )
            .unwrap();
        }
        s.evaluate("run", None, &[Json::Int(1)], &mut remote)
            .unwrap();
        let before = s.take_trace();
        assert!(!before.is_empty(), "multi-model run produced no trace");
        assert!(before.iter().any(|line| line.contains(" COUNT ")));

        s.evaluate(
            "define-chunks",
            Some("COUNT"),
            &[Json::Arr(vec![
                Json::Str("RUNTIME-CHUNK".into()),
                Json::Str("ISA".into()),
                Json::Str("CHUNK".into()),
            ])],
            &mut remote,
        )
        .unwrap();
        let after = s.take_trace();
        assert!(
            after.is_empty(),
            "define-chunks replayed the run trace: {after:?}"
        );
        let chunk_p = s
            .evaluate(
                "chunk-p",
                Some("COUNT"),
                &[Json::Str("RUNTIME-CHUNK".into())],
                &mut remote,
            )
            .unwrap();
        assert_eq!(chunk_p[0], Json::Bool(true));
    }

    #[test]
    fn multi_model_run_interleaves_and_switches_viewers() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(2)], &mut r).unwrap();
        let trace = s.take_trace();
        assert!(
            trace.iter().any(|l| l.contains("COUNT")),
            "count model missing from multi trace"
        );
        assert!(
            trace.iter().any(|l| l.contains("ADDITION")),
            "addition model missing from multi trace"
        );

        let count_prods = s
            .evaluate("all-productions", Some("COUNT"), &[], &mut r)
            .unwrap();
        let addition_prods = s
            .evaluate("all-productions", Some("ADDITION"), &[], &mut r)
            .unwrap();
        assert!(matches!(count_prods.first(), Some(Json::Arr(a)) if !a.is_empty()));
        assert!(matches!(addition_prods.first(), Some(Json::Arr(a)) if !a.is_empty()));
        let count_chunks = s
            .evaluate(
                "filter-dm-chunks",
                Some("COUNT"),
                &[Json::Str("_none_".into())],
                &mut r,
            )
            .unwrap();
        let addition_chunks = s
            .evaluate(
                "filter-dm-chunks",
                Some("ADDITION"),
                &[Json::Str("_none_".into())],
                &mut r,
            )
            .unwrap();
        assert_ne!(count_chunks, addition_chunks);
        s.evaluate("reset", None, &[], &mut r).unwrap();
        s.evaluate(
            "record-history",
            Some("COUNT"),
            &[Json::Str("retrieval-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(2)], &mut r).unwrap();
        let count_history = s
            .evaluate(
                "history-data-available",
                Some("COUNT"),
                &[Json::Str("retrieval-history".into())],
                &mut r,
            )
            .unwrap();
        let addition_history = s
            .evaluate(
                "history-data-available",
                Some("ADDITION"),
                &[Json::Str("retrieval-history".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(count_history[0], Json::Bool(true));
        assert_eq!(addition_history[0], Json::Bool(false));
        assert_eq!(
            s.evaluate("current-model", None, &[], &mut r).unwrap(),
            vec![Json::Str("ADDITION".into())]
        );
    }

    #[test]
    fn multi_model_honors_per_model_trace_and_parameter_types() {
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.load_source(
            r#"
(define-model trace-a
  (sgp :v t :seed (101 0))
  (chunk-type goal state)
  (add-dm (start isa goal state 0))
  (goal-focus start))
(define-model trace-b
  (sgp :v nil :seed (202 0))
  (chunk-type goal state)
  (add-dm (start isa goal state 0))
  (goal-focus start))
"#,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Float(0.1)], &mut r)
            .unwrap();
        let trace = s.take_trace();
        assert!(
            trace.iter().any(|line| line.contains("TRACE-A")),
            "trace-on model missing: {trace:?}"
        );
        assert!(
            !trace.iter().any(|line| line.contains("TRACE-B")),
            "trace-off model leaked into the trace: {trace:?}"
        );

        let seed = s
            .evaluate(
                "get-parameter-value",
                Some("TRACE-A"),
                &[Json::Str(":seed".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(seed, vec![Json::Arr(vec![Json::Int(101), Json::Int(0)])]);
        let v = s
            .evaluate(
                "get-parameter-value",
                Some("TRACE-B"),
                &[Json::Str(":v".into())],
                &mut r,
            )
            .unwrap();
        assert_eq!(v, vec![Json::Null]);
    }

    #[test]
    fn multi_buffer_goal_chunks_are_exposed() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("extras;threads;sample.lisp"))],
            &mut remote,
        )
        .unwrap();
        let multi = s
            .evaluate(
                "multi-buffer-p",
                None,
                &[Json::Str("GOAL".into())],
                &mut remote,
            )
            .unwrap();
        assert_eq!(multi[0], Json::Bool(true));
        let chunks = s
            .evaluate(
                "get-m-buffer-chunks",
                None,
                &[Json::Str("GOAL".into())],
                &mut remote,
            )
            .unwrap();
        assert!(matches!(chunks.first(), Some(Json::Arr(a)) if !a.is_empty()));
    }

    #[test]
    fn pdisable_and_penable_toggle_productions() {
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
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
        let root = fixture_root();
        if !root.is_dir() {
            return; // vendored fixtures not available
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model(
                "test-models;models;safe-queries.lisp",
            ))],
            &mut remote,
        )
        .unwrap();
        let _ = s.take_trace();
        s.evaluate("pp", None, &[], &mut remote).unwrap();
        let got = s.take_trace();

        // `safe-queries.ref` records two `(pp)` blocks; the first is the plain
        // definition order, which is what a fresh `pp` prints.
        let ref_txt = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("cases")
                .join("safe-queries.ref"),
        )
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
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = Collecting::default();
        s.add_remote("sink", None, Some("me".into()));
        s.add_monitor("model-trace", "sink", MonitorWhen::Simple);
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
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
        let lines = s.take_trace_events();
        for (l, kind) in lines {
            s.deliver_trace(&l, kind, &mut remote);
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
        assert!(remote.lines.iter().all(|l| l.ends_with('\n')));
    }

    #[test]
    fn dm_and_sdm_print_and_return_names() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
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
        let mut s = Session::new((100, 0));
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model(
                "tutorial;unit1;tutor-model-solution.lisp",
            ))],
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
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
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
    fn run_returns_elapsed_time_and_get_time_returns_ms() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut remote,
        )
        .unwrap();
        s.evaluate("reset", None, &[], &mut remote).unwrap();

        let n2 = s
            .evaluate("run-n-events", None, &[Json::Int(2)], &mut remote)
            .unwrap();
        assert_eq!(n2, vec![Json::Float(0.0), Json::Int(2), Json::Null]);
        let time = s
            .evaluate("get-time", None, &[Json::Bool(true)], &mut remote)
            .unwrap();
        assert_eq!(time, vec![Json::Int(0)]);

        let until = s
            .evaluate("run-until-time", None, &[Json::Float(0.3)], &mut remote)
            .unwrap();
        assert_eq!(until[0], Json::Float(0.3));
        assert!(matches!(until[1], Json::Int(n) if n > 0));
        assert_eq!(until[2], Json::Null);
        let time = s
            .evaluate("get-time", None, &[Json::Bool(true)], &mut remote)
            .unwrap();
        assert_eq!(time, vec![Json::Int(300)]);
    }

    #[test]
    fn goal_focus_accepts_a_spec_list() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut remote = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;semantic.lisp"))],
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

    #[test]
    fn trace_history_records_carry_detail_and_model() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace();

        let all = s.history_records("TRACE-HISTORY").unwrap();
        let low_req = Json::Str("low".into());
        let low = filter_history_records(all.clone(), None, None, Some(&low_req));
        assert!(!low.is_empty());
        for rec in &low {
            let Json::Arr(fields) = crate::json::parse(rec).unwrap() else {
                panic!("low trace record is not an array");
            };
            assert_eq!(fields[1].as_str(), Some("low"));
        }
        let med_req = Json::Str("medium".into());
        let medium = filter_history_records(all.clone(), None, None, Some(&med_req));
        assert!(medium.len() >= low.len());
        for rec in &medium {
            let Json::Arr(fields) = crate::json::parse(rec).unwrap() else {
                panic!("medium trace record is not an array");
            };
            assert!(matches!(fields[1].as_str(), Some("low" | "medium")));
        }

        let id = s
            .evaluate(
                "start-incremental-history-data",
                None,
                &[Json::Str("trace-history".into()), Json::Int(16000)],
                &mut r,
            )
            .unwrap()[0]
            .clone();
        let chunk = s
            .evaluate("get-incremental-history-data", None, &[id], &mut r)
            .unwrap();
        let Json::Arr(records) = crate::json::parse(chunk[0].as_str().unwrap()).unwrap() else {
            panic!("trace history is not an array");
        };
        assert!(!records.is_empty());
        let mut saw_low = false;
        let mut saw_medium = false;
        for rec in records {
            let Json::Arr(fields) = rec else {
                panic!("trace record is not an array");
            };
            assert_eq!(fields.len(), 4);
            let detail = fields[1].as_str().expect("trace detail");
            assert!(matches!(detail, "low" | "medium" | "high"));
            assert_eq!(fields[2].as_str(), Some("COUNT"));
            assert!(fields[3].as_str().is_some_and(|s| !s.is_empty()));
            saw_low |= detail == "low";
            saw_medium |= detail == "medium";
        }
        assert!(saw_low, "count trace should contain low-detail lines");
        assert!(saw_medium, "count trace should contain medium-detail lines");
    }

    #[test]
    fn multi_model_trace_history_carries_each_model_name() {
        let root = fixture_root();
        if !root.is_dir() {
            return;
        }
        let mut s = Session::new((100, 0));
        let mut r = NoRemote;
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;count.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "load-act-r-model",
            None,
            &[Json::Str(fixture_model("tutorial;unit1;addition.lisp"))],
            &mut r,
        )
        .unwrap();
        s.evaluate(
            "record-history",
            None,
            &[Json::Str("trace-history".into())],
            &mut r,
        )
        .unwrap();
        s.evaluate("run", None, &[Json::Int(10)], &mut r).unwrap();
        let _ = s.take_trace();

        let records = s.history_records("TRACE-HISTORY").unwrap();
        let mut models = std::collections::BTreeSet::new();
        for rec in records {
            let Json::Arr(fields) = crate::json::parse(&rec).unwrap() else {
                panic!("trace record is not an array");
            };
            models.insert(fields[2].as_str().unwrap_or("").to_string());
        }
        assert!(models.contains("COUNT"), "{models:?}");
        assert!(models.contains("ADDITION"), "{models:?}");
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
