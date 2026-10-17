//! In-process backend: the GUI owns the engine [`Session`] directly.
//!
//! This is the default `actr env` wiring. Commands go through
//! [`Session::evaluate`], so the exact same dispatch the RPC server uses is
//! exercised; trace is drained from the session's `out_lines`, and visible
//! window display ops are collected from the client handler's `Remote`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::json::Json;
use crate::rpc::session::{Remote, Session};

use super::backend::EnvBackend;

/// A [`Remote`] for the client-owned visible-window handler.
///
/// The engine calls it (via `Session::evaluate`) with each display op; `check`
/// is answered `true` (the GUI always displays), everything else is queued for
/// the UI thread. Any other client command is unexpected and reported.
struct GuiRemote {
    handler: String,
    ops: Arc<Mutex<Vec<Vec<Json>>>>,
}

impl Remote for GuiRemote {
    fn call(
        &mut self,
        name: &str,
        _model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        if name != self.handler {
            return Err(format!("no in-process handler for client command {name:?}"));
        }
        let check = args
            .first()
            .and_then(|a| a.as_str())
            .map(|s| s == "check")
            .unwrap_or(false);
        if !check {
            self.ops.lock().unwrap().push(args);
        }
        Ok(vec![Json::Bool(true)])
    }
}

/// The GUI owns the session and drives it on the UI thread.
pub struct InProc {
    session: Session,
    handler: String,
    ops: Arc<Mutex<Vec<Vec<Json>>>>,
}

impl InProc {
    pub fn new(root: PathBuf, seed: (u64, u64)) -> InProc {
        InProc {
            session: Session::new(root, seed),
            handler: String::new(),
            ops: Arc::new(Mutex::new(Vec::new())),
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
        let mut remote = GuiRemote {
            handler: self.handler.clone(),
            ops: self.ops.clone(),
        };
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

    fn register_window_handler(&mut self, name: &str) {
        self.handler = name.to_string();
        self.session.add_remote(name, Some(name.to_string()), None);
    }

    fn take_window_ops(&mut self) -> Vec<Vec<Json>> {
        std::mem::take(&mut *self.ops.lock().unwrap())
    }
}
