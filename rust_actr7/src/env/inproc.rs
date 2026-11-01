//! In-process backend: the GUI owns the engine [`Session`] directly.
//!
//! This is the default `actr env` wiring. Commands go through
//! [`Session::evaluate`], so the exact same dispatch the RPC server uses is
//! exercised; trace is drained from the session's `out_lines`, and visible
//! window display ops are collected from the client handler's `Remote`.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::json::Json;
use crate::rpc::session::{Remote, Session};

use super::backend::EnvBackend;

/// An in-progress interactive run.
struct RunState {
    limit_ms: i64,
    full: bool,
    /// Model time (ms) at the run's start, for real-time pacing.
    start_model_ms: i64,
    wall_start: Instant,
    real_time: bool,
}

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
    run: Option<RunState>,
    stop: bool,
    /// Pace runs to wall-clock time (`run`'s real-time flag).
    real_time: bool,
}

impl InProc {
    pub fn new(seed: (u64, u64)) -> InProc {
        InProc {
            session: Session::new(seed),
            handler: String::new(),
            ops: Arc::new(Mutex::new(Vec::new())),
            run: None,
            stop: false,
            real_time: true,
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

    fn begin_run(&mut self, secs: f64, full: bool) -> bool {
        let name = if full { "run-full-time" } else { "run" };
        let start_model_ms = (self.session.time_secs() * 1000.0).round() as i64;
        let (limit_ms, full, should) = self.session.run_begin(name, secs);
        self.stop = false;
        if should {
            self.run = Some(RunState {
                limit_ms,
                full,
                start_model_ms,
                wall_start: Instant::now(),
                real_time: self.real_time,
            });
            true
        } else {
            self.run = None;
            false
        }
    }

    fn poll_run(&mut self) -> Result<bool, String> {
        if self.stop {
            // Drop the run (no terminator; the user aborted it).
            self.stop = false;
            self.run = None;
            return Ok(false);
        }
        let Some(run) = &self.run else {
            return Ok(false);
        };
        // Real-time pacing: cap the model time at wall-clock elapsed; else
        // just ~200 events per frame to keep the UI responsive.
        let soft = if run.real_time {
            let elapsed = run.wall_start.elapsed().as_millis() as i64;
            Some(run.start_model_ms + elapsed)
        } else {
            None
        };
        let running = self
            .session
            .run_chunk(run.limit_ms, run.full, Some(200), soft)?;
        if !running {
            self.run = None;
        }
        Ok(running)
    }

    fn stop_run(&mut self) {
        self.stop = true;
    }

    fn is_running(&self) -> bool {
        self.run.is_some()
    }

    fn set_real_time(&mut self, on: bool) {
        self.real_time = on;
    }
}
