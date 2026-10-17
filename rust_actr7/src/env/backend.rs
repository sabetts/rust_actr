//! The transport-neutral surface the Environment GUI panels talk to.
//!
//! Both the in-process backend ([`super::inproc::InProc`]) and the RPC-client
//! backend ([`super::client::Client`]) implement [`EnvBackend`], so panels
//! issue plain ACT-R command strings and never care where the engine lives.

use crate::json::Json;

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

    /// Drain trace lines accumulated since the last call (the four
    /// `framework/printing.lisp` streams share one sink here).
    fn take_trace(&mut self) -> Vec<String>;

    /// The current model's name, if one is loaded.
    fn current_model(&self) -> Option<String>;

    /// Select the current model (`set-current-model`).
    fn set_current_model(&mut self, name: Option<String>);

    /// A short description of the backend for the status bar.
    fn describe(&self) -> String;

    /// Register `name` as the client command the engine calls with
    /// visible-virtual window ops (`add-virtual-window-handler`'s handler).
    fn register_window_handler(&mut self, name: &str);

    /// Drain the display ops the engine pushed to the window handler since
    /// the last call (each is `[verb, …args]`).
    fn take_window_ops(&mut self) -> Vec<Vec<Json>>;

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
