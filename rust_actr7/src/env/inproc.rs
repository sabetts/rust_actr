//! In-process backend: the GUI owns the engine [`Session`] directly.
//!
//! This is the default `actr env` wiring. Commands go through
//! [`Session::evaluate`], so the exact same dispatch the RPC server uses is
//! exercised; trace, signals, window ops, and stepper callbacks are collected
//! into a single [`BackendEvent`] stream for the UI thread.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::engine::TraceKind;
use crate::json::Json;
use crate::rpc::session::{Remote, Session};

use super::backend::{BackendEvent, ConnectionInfo, EnvBackend, TraceStream};

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
    /// Events waiting for the UI thread.
    events: Vec<BackendEvent>,
    /// Last model list, so load/clear changes become model events.
    known_models: Vec<String>,
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
            events: Vec::new(),
            known_models: Vec::new(),
        }
    }

    /// Direct access to the session (run slicing, device actions, …).
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Gather trace, signals, window ops, and stepper callbacks into the event
    /// queue.
    fn collect_events(&mut self) {
        for op in std::mem::take(&mut *self.ops.lock().unwrap()) {
            self.events.push(BackendEvent::WindowOp(op));
        }
        for (line, kind) in self.session.take_trace_events() {
            let stream = match kind {
                TraceKind::Model => TraceStream::Model,
                TraceKind::Command => TraceStream::Command,
                TraceKind::Warning => TraceStream::Warning,
                TraceKind::General => TraceStream::General,
            };
            self.events.push(BackendEvent::Trace { stream, line });
        }
        // The session reports load/reload/clear changes through the model
        // table. Diff it so in-process mode emits the same create/delete
        // events the RPC monitors would.
        let current = self.session.models();
        for name in &current {
            if !self.known_models.contains(name) {
                self.events.push(BackendEvent::ModelCreated(name.clone()));
            }
        }
        for name in &self.known_models {
            if !current.contains(name) {
                self.events.push(BackendEvent::ModelDeleted(name.clone()));
            }
        }
        self.known_models = current;
        for (name, model, _args) in self.session.take_pending_signals() {
            match name.as_str() {
                // Run signals are synthesized around begin_run/poll_run; model
                // changes are diffed above.
                "creating-model" | "deleting-model" | "run-start" | "run-stop" => {}
                other => self
                    .events
                    .push(BackendEvent::Status(format!("signal: {other} {model:?}"))),
            }
        }
        while let Some((command, model, args)) = self.session.take_stepper_pause() {
            self.events.push(BackendEvent::StepperPause {
                model,
                command,
                args,
            });
        }
        while let Some((command, model, args)) = self.session.take_stepper_update() {
            self.events.push(BackendEvent::StepperUpdate {
                model,
                command,
                args,
            });
        }
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
        let result = self.session.evaluate(name, model, &args, &mut remote);
        self.collect_events();
        result
    }

    fn take_events(&mut self) -> Vec<BackendEvent> {
        self.collect_events();
        std::mem::take(&mut self.events)
    }

    fn current_model(&self) -> Option<String> {
        self.session.current_model().map(str::to_string)
    }

    fn models(&self) -> Vec<String> {
        self.session.models()
    }

    fn set_current_model(&mut self, name: Option<String>) {
        self.session.set_current_model(name);
        self.collect_events();
    }

    fn describe(&self) -> String {
        "in-process".to_string()
    }

    fn register_window_handler(&mut self, name: &str) {
        self.handler = name.to_string();
        self.session.add_remote(name, Some(name.to_string()), None);
    }

    fn register_stepper_handlers(&mut self, next: &str, update: &str) -> Result<(), String> {
        // The session only needs these names to exist; in-process mode drains
        // the shared pause/update state directly in `collect_events`.
        self.session.add_remote(next, Some(next.to_string()), None);
        self.session
            .add_remote(update, Some(update.to_string()), None);
        Ok(())
    }

    fn connections(&mut self) -> Result<Vec<ConnectionInfo>, String> {
        Ok(vec![ConnectionInfo {
            name: "in-process".into(),
            address: "local".into(),
            ..Default::default()
        }])
    }

    fn command_doc(&mut self, name: &str) -> Result<Option<String>, String> {
        Ok(self.session.command_doc(name))
    }

    fn begin_run(&mut self, secs: f64, full: bool) -> bool {
        let name = if full { "run-full-time" } else { "run" };
        let start_model_ms = (self.session.time_secs() * 1000.0).round() as i64;
        let start_time = self.session.time_secs();
        let model = self.current_model();
        let (limit_ms, full, should) = self.session.run_begin(name, secs);
        self.stop = false;
        if should {
            self.events.push(BackendEvent::RunStarted {
                model,
                time: start_time,
            });
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
            let model = self.current_model();
            let time = self.session.time_secs();
            let events = self.session.run_events();
            self.collect_events();
            self.events.push(BackendEvent::RunStopped {
                model,
                time,
                events,
            });
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
        // A stepper pause withholds the event and returns like a capped run;
        // keep the interactive run alive until the GUI calls `step-stepper`.
        let paused = self.session.stepper_paused();
        let model = self.current_model();
        let time = self.session.time_secs();
        let events = self.session.run_events();
        self.collect_events();
        if !running && !paused {
            self.events.push(BackendEvent::RunStopped {
                model,
                time,
                events,
            });
            self.run = None;
        }
        Ok(running || paused)
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
