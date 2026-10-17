//! In-process backend: the GUI owns the engine [`Session`] directly.
//!
//! This is the default `actr env` wiring. Commands go through
//! [`Session::evaluate`], so the exact same dispatch the RPC server uses is
//! exercised; trace is drained from the session's `out_lines`.

use std::path::PathBuf;

use crate::json::Json;
use crate::rpc::session::{Remote, Session};

use super::backend::EnvBackend;

/// A [`Remote`] for client-owned commands (monitors, virtual-window handlers).
///
/// In-process the GUI registers nothing with the dispatcher yet, so any call
/// back to a "client" command is unexpected and reported as an error rather
/// than silently dropped. Later phases hand this a queue of display ops.
struct GuiRemote;

impl Remote for GuiRemote {
    fn call(
        &mut self,
        name: &str,
        _model: Option<&str>,
        _args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        Err(format!("no in-process handler for client command {name:?}"))
    }
}

/// The GUI owns the session and drives it on the UI thread.
pub struct InProc {
    session: Session,
}

impl InProc {
    pub fn new(root: PathBuf, seed: (u64, u64)) -> InProc {
        InProc {
            session: Session::new(root, seed),
        }
    }

    /// Direct access to the session (run slicing, device actions, …).
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

impl EnvBackend for InProc {
    fn evaluate(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        let mut remote = GuiRemote;
        self.session.evaluate(name, model, &args, &mut remote)
    }

    fn take_trace(&mut self) -> Vec<String> {
        self.session.take_trace()
    }

    fn current_model(&self) -> Option<String> {
        self.session.current_model().map(str::to_string)
    }

    fn set_current_model(&mut self, name: Option<String>) {
        self.session.set_current_model(name);
    }

    fn describe(&self) -> String {
        "in-process".to_string()
    }
}
