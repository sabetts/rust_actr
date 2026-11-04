//! The transport-neutral surface the Environment GUI panels talk to.
//!
//! Both the in-process backend ([`super::inproc::InProc`]) and the RPC-client
//! backend ([`super::client::Client`]) implement [`EnvBackend`], so panels
//! issue plain ACT-R command strings and never care where the engine lives.

use crate::json::Json;

/// One of ACT-R's four output streams (`framework/printing.lisp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceStream {
    Model,
    Command,
    Warning,
    General,
}

impl TraceStream {
    pub fn name(self) -> &'static str {
        match self {
            TraceStream::Model => "model-trace",
            TraceStream::Command => "command-trace",
            TraceStream::Warning => "warning-trace",
            TraceStream::General => "general-trace",
        }
    }
}

/// Transport-neutral events a backend produces for the GUI.
#[derive(Debug, Clone)]
pub enum BackendEvent {
    /// A trace line on one of the four ACT-R output streams.
    Trace { stream: TraceStream, line: String },
    /// `creating-model`: a model was added to the session.
    ModelCreated(String),
    /// `deleting-model`: a model was removed from the session.
    ModelDeleted(String),
    /// `run-start`.
    RunStarted { model: Option<String>, time: f64 },
    /// `run-stop`.
    RunStopped {
        model: Option<String>,
        time: f64,
        events: u64,
    },
    /// A visible experiment-window display op (`[verb, …]`).
    WindowOp(Vec<Json>),
    /// A stepper pre-event pause callback (`next-event`).
    StepperPause {
        model: Option<String>,
        command: String,
        args: Vec<Json>,
    },
    /// A stepper post-event update callback (`update-event`).
    StepperUpdate {
        model: Option<String>,
        command: String,
        args: Vec<Json>,
    },
    /// A human-readable status/error message from the backend.
    Status(String),
}

/// A connection row from the dispatcher (`list-connections`).
#[derive(Debug, Clone, Default)]
pub struct ConnectionInfo {
    pub name: String,
    pub address: String,
    pub pending_calls: i64,
    pub sent: i64,
    pub received: i64,
    pub commands: Vec<String>,
}

/// A connection to an ACT-R engine that the GUI drives.
pub trait EnvBackend {
    /// Evaluate an ACT-R command by name (`evaluate <name> <model> <args…>`),
    /// exactly as the Tcl dialogs' `call_act_r_command` does.
    fn evaluate(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String>;

    /// Drain events accumulated since the last call.
    fn take_events(&mut self) -> Vec<BackendEvent>;

    /// The current model's name, if one is loaded.
    fn current_model(&self) -> Option<String>;

    /// Models in load order (`mp-models`).
    fn models(&self) -> Vec<String>;

    /// Select the current model (`set-current-model`).
    fn set_current_model(&mut self, name: Option<String>);

    /// A short description of the backend for the status bar.
    fn describe(&self) -> String;

    /// Register `name` as the client command the engine calls with
    /// visible-virtual window ops (`add-virtual-window-handler`'s handler).
    fn register_window_handler(&mut self, name: &str);

    /// Register the two client-owned callbacks the server invokes for the
    /// stepper (`next-event` and `update-event`).
    fn register_stepper_handlers(&mut self, next: &str, update: &str) -> Result<(), String>;

    /// Connections known to the dispatcher (client mode) or the local
    /// in-process backend.
    fn connections(&mut self) -> Result<Vec<ConnectionInfo>, String> {
        Ok(Vec::new())
    }

    /// The documentation string for a registered command (`check <name>`).
    fn command_doc(&mut self, name: &str) -> Result<Option<String>, String>;

    /// Start an interactive run of `secs` model seconds (`full` = the
    /// `run-full-time` terminator). Returns whether a run started.
    fn begin_run(&mut self, secs: f64, full: bool) -> bool;

    /// Advance an interactive run by one chunk. `Ok(true)` while it is still
    /// going; the caller should call again next frame.
    fn poll_run(&mut self) -> Result<bool, String>;

    /// Request the current interactive run stop.
    fn stop_run(&mut self);

    /// Whether an interactive run is in progress.
    fn is_running(&self) -> bool;

    /// Enable/disable real-time pacing of interactive runs.
    fn set_real_time(&mut self, on: bool);
}

/// Render a command result the way a dialog would show it: `~A`-style items
/// joined by spaces, with nested lists in parenthesised ACT-R form.
pub fn result_text(values: &[Json]) -> String {
    values
        .iter()
        .map(crate::rpc::session::json_to_text)
        .collect::<Vec<_>>()
        .join(" ")
}
