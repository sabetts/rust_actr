//! RPC-client backend: the GUI is a real ACT-R dispatcher client.
//!
//! Speaks the same `\x04`-framed JSON protocol as `tutorial/python/actr.py`
//! and the Tcl environment: a background reader thread matches reply `id`s to
//! pending requests and services server→client `evaluate` callbacks (trace
//! monitors, model/run signals, and visible-window ops).

use std::collections::HashMap;
use std::io::BufReader;
use std::net::TcpStream;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::json::Json;
use crate::rpc::{
    parse_reply, parse_request, read_message, reply_err, reply_ok, write_message, Method,
};

use super::backend::{BackendEvent, ConnectionInfo, EnvBackend, TraceStream};

/// One command per trace stream, so the UI can tag lines by source.
const TRACE_HANDLERS: [(&str, &str, TraceStream); 4] = [
    ("model-trace", "env-model-trace", TraceStream::Model),
    ("command-trace", "env-command-trace", TraceStream::Command),
    ("warning-trace", "env-warning-trace", TraceStream::Warning),
    ("general-trace", "env-general-trace", TraceStream::General),
];
/// The visible-window handler command.
const VV_HANDLER: &str = "env-vv-handler";
/// Signal handlers registered as client-owned commands.
const MODEL_CREATED: &str = "env-model-created";
const MODEL_DELETED: &str = "env-model-deleted";
const RUN_START: &str = "env-run-start";
const RUN_STOP: &str = "env-run-stop";
/// Stepper callback handlers.
const STEP_NEXT: &str = "env-next-event";
const STEP_UPDATE: &str = "env-update-event";

type Pending = Arc<Mutex<HashMap<i64, Sender<Result<Vec<Json>, String>>>>>;

pub struct Client {
    writer: Arc<Mutex<TcpStream>>,
    pending: Pending,
    events: Arc<Mutex<Vec<BackendEvent>>>,
    next_id: AtomicI64,
    /// Cached `current-model` (queried on demand; the trait getter is `&self`).
    model: Option<String>,
    /// Cached `mp-models` order.
    models: Vec<String>,
    addr: String,
    /// A run in flight. The reply arrives on the reader thread, so the UI
    /// thread can poll it and still send Stop on the same connection.
    run: Option<mpsc::Receiver<Result<Vec<Json>, String>>>,
}

impl Client {
    /// Connect to a running `actr server` and register trace/signal monitors.
    pub fn connect(host: &str, port: u16) -> Result<Client, String> {
        let stream = TcpStream::connect((host, port))
            .map_err(|e| format!("could not connect to {host}:{port}: {e}"))?;
        let read_stream = stream.try_clone().map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(stream));
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let events: Arc<Mutex<Vec<BackendEvent>>> = Arc::new(Mutex::new(Vec::new()));

        spawn_reader(
            BufReader::new(read_stream),
            writer.clone(),
            pending.clone(),
            events.clone(),
        );

        let mut client = Client {
            writer,
            pending,
            events,
            next_id: AtomicI64::new(1),
            model: None,
            models: Vec::new(),
            addr: format!("{host}:{port}"),
            run: None,
        };

        client.request("set-name", vec![Json::Str("ACT-R Environment".into())])?;
        for (stream, handler, _) in TRACE_HANDLERS {
            client.register_handler(stream, handler, "Environment trace sink")?;
        }
        for (signal, handler) in [
            ("creating-model", MODEL_CREATED),
            ("deleting-model", MODEL_DELETED),
            ("run-start", RUN_START),
            ("run-stop", RUN_STOP),
        ] {
            client.register_handler(signal, handler, "Environment signal handler")?;
        }
        let _ = client.refresh_models();
        let _ = client.refresh_model();
        Ok(client)
    }

    /// Register a client-owned command and attach it as a simple monitor.
    fn register_handler(&self, target: &str, handler: &str, doc: &str) -> Result<(), String> {
        // Register the handler as a client-owned command, then attach it as a
        // simple monitor of the stream/signal command.
        self.request(
            "add",
            vec![
                Json::Str(handler.into()),
                Json::Str(handler.into()),
                Json::Str(doc.into()),
                Json::Null,
            ],
        )?;
        self.request(
            "monitor",
            vec![
                Json::Str(target.into()),
                Json::Str(handler.into()),
                Json::Str("simple".into()),
            ],
        )?;
        Ok(())
    }

    /// Send a request and return a receiver for its reply.
    fn start_request(
        &self,
        method: &str,
        params: Vec<Json>,
    ) -> Result<mpsc::Receiver<Result<Vec<Json>, String>>, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id, tx);
        let msg = Json::Obj(vec![
            ("method".into(), Json::Str(method.into())),
            ("params".into(), Json::Arr(params)),
            ("id".into(), Json::Int(id)),
        ]);
        {
            let mut w = self.writer.lock().unwrap();
            write_message(&mut *w, &msg).map_err(|e| e.to_string())?;
        }
        Ok(rx)
    }

    /// Send a request and block for its reply.
    fn request(&self, method: &str, params: Vec<Json>) -> Result<Vec<Json>, String> {
        let rx = self.start_request(method, params)?;
        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(r) => r,
            Err(_) => {
                // The id is not known here; the reader will drop the sender
                // when the reply finally arrives.
                Err("timed out waiting for the server".into())
            }
        }
    }

    /// Refresh the cached current-model name from the server.
    pub fn refresh_model(&mut self) -> Option<String> {
        if let Ok(v) = self.request(
            "evaluate",
            vec![Json::Str("current-model".into()), Json::Null],
        ) {
            match v.first() {
                Some(Json::Str(name)) => self.model = Some(name.clone()),
                Some(Json::Null) | None => self.model = None,
                _ => {}
            }
        }
        self.model.clone()
    }

    /// Refresh the cached model order from the server.
    pub fn refresh_models(&mut self) -> Vec<String> {
        if let Ok(v) = self.request("evaluate", vec![Json::Str("mp-models".into()), Json::Null]) {
            if let Some(Json::Arr(names)) = v.first() {
                self.models = names
                    .iter()
                    .filter_map(|n| n.as_str().map(str::to_string))
                    .collect();
            }
        }
        self.models.clone()
    }
}

impl EnvBackend for Client {
    fn evaluate(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        let mut params = vec![
            Json::Str(name.to_string()),
            model
                .map(|m| Json::Str(m.to_string()))
                .unwrap_or(Json::Null),
        ];
        params.extend(args);
        let result = self.request("evaluate", params);
        // Model selection / loading changes the cached name.
        if matches!(
            name,
            "load-act-r-model"
                | "load-act-r-code"
                | "reset"
                | "set-current-model"
                | "reload"
                | "clear-all"
        ) {
            let _ = self.refresh_model();
            let _ = self.refresh_models();
        }
        result
    }

    fn take_events(&mut self) -> Vec<BackendEvent> {
        let events = std::mem::take(&mut *self.events.lock().unwrap());
        if events.iter().any(|e| {
            matches!(
                e,
                BackendEvent::ModelCreated(_) | BackendEvent::ModelDeleted(_)
            )
        }) {
            let _ = self.refresh_model();
            let _ = self.refresh_models();
        }
        events
    }

    fn current_model(&self) -> Option<String> {
        self.model.clone()
    }

    fn models(&self) -> Vec<String> {
        self.models.clone()
    }

    fn set_current_model(&mut self, name: Option<String>) {
        let arg = name.clone().map(Json::Str).unwrap_or(Json::Null);
        let _ = self.evaluate("set-current-model", None, vec![arg]);
        self.model = name;
    }

    fn describe(&self) -> String {
        format!("RPC client → {}", self.addr)
    }

    fn register_window_handler(&mut self, name: &str) {
        let _ = self.request(
            "add",
            vec![
                Json::Str(name.to_string()),
                Json::Str(name.to_string()),
                Json::Str("Environment visible-window handler".into()),
                Json::Null,
                Json::Str(name.to_string()),
            ],
        );
    }

    fn register_stepper_handlers(&mut self, next: &str, update: &str) -> Result<(), String> {
        for name in [next, update] {
            self.request(
                "add",
                vec![
                    Json::Str(name.to_string()),
                    Json::Str(name.to_string()),
                    Json::Str("Environment stepper callback".into()),
                    Json::Null,
                ],
            )?;
        }
        Ok(())
    }

    fn command_doc(&mut self, name: &str) -> Result<Option<String>, String> {
        let v = self.request("check", vec![Json::Str(name.to_string())])?;
        match v.as_slice() {
            [Json::Null] | [] => Ok(None),
            [_, _, Json::Str(doc), ..] => Ok(Some(doc.clone())),
            _ => Ok(None),
        }
    }

    fn connections(&mut self) -> Result<Vec<ConnectionInfo>, String> {
        let v = self.request("list-connections", Vec::new())?;
        let Some(Json::Arr(rows)) = v.first() else {
            return Ok(Vec::new());
        };
        Ok(rows
            .iter()
            .filter_map(|row| {
                let Json::Arr(f) = row else { return None };
                Some(ConnectionInfo {
                    name: f.first().and_then(|x| x.as_str()).unwrap_or("").into(),
                    address: String::new(),
                    pending_calls: f.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as i64,
                    sent: f.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0) as i64,
                    received: f.get(3).and_then(|x| x.as_f64()).unwrap_or(0.0) as i64,
                    commands: f
                        .get(4)
                        .and_then(|x| match x {
                            Json::Arr(a) => Some(
                                a.iter()
                                    .filter_map(|s| s.as_str().map(str::to_string))
                                    .collect(),
                            ),
                            _ => None,
                        })
                        .unwrap_or_default(),
                })
            })
            .collect())
    }

    fn begin_run(&mut self, secs: f64, full: bool) -> bool {
        let name = if full { "run-full-time" } else { "run" };
        match self.start_request(
            "evaluate",
            vec![Json::Str(name.into()), Json::Null, Json::Float(secs)],
        ) {
            Ok(rx) => {
                self.run = Some(rx);
                true
            }
            Err(e) => {
                self.run = None;
                let _ = self.events.lock().unwrap().push(BackendEvent::Status(e));
                false
            }
        }
    }

    fn poll_run(&mut self) -> Result<bool, String> {
        let Some(rx) = &self.run else {
            return Ok(false);
        };
        match rx.try_recv() {
            Ok(Ok(_)) => {
                self.run = None;
                let _ = self.refresh_model();
                let _ = self.refresh_models();
                Ok(false)
            }
            Ok(Err(e)) => {
                self.run = None;
                Err(e)
            }
            Err(mpsc::TryRecvError::Empty) => Ok(true),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.run = None;
                Err("connection closed while a run was in progress".into())
            }
        }
    }

    fn stop_run(&mut self) {
        // The server pumps client requests even while `run` is in flight, so
        // this break reaches the same session and stops it before the limit.
        let _ = self.request(
            "evaluate",
            vec![
                Json::Str("schedule-break-relative".into()),
                Json::Null,
                Json::Int(0),
            ],
        );
    }

    fn is_running(&self) -> bool {
        self.run.is_some()
    }

    fn set_real_time(&mut self, _on: bool) {}
}

/// The reader loop: replies to pending requests, trace/signal callbacks, and
/// visible-window display ops.
fn spawn_reader(
    mut reader: BufReader<TcpStream>,
    writer: Arc<Mutex<TcpStream>>,
    pending: Pending,
    events: Arc<Mutex<Vec<BackendEvent>>>,
) {
    thread::spawn(move || loop {
        let msg = match read_message(&mut reader) {
            Ok(Some(m)) => m,
            _ => break,
        };
        if msg.get("method").is_some() {
            if let Ok((method, params, id)) = parse_request(&msg) {
                if method == Method::Evaluate {
                    let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
                    let model = params.get(1).and_then(|p| match p {
                        Json::Str(s) => Some(s.clone()),
                        _ => None,
                    });
                    let args: Vec<Json> = params.iter().skip(2).cloned().collect();
                    let trace = TRACE_HANDLERS
                        .iter()
                        .find(|(_, handler, _)| *handler == name)
                        .map(|(_, _, stream)| *stream);
                    let handled = {
                        let mut out = events.lock().unwrap();
                        if let Some(stream) = trace {
                            if let Some(line) = args.first().and_then(|p| p.as_str()) {
                                out.push(BackendEvent::Trace {
                                    stream,
                                    line: line.to_string(),
                                });
                            }
                            true
                        } else {
                            match name {
                                MODEL_CREATED => {
                                    if let Some(n) = args.first().and_then(|p| p.as_str()) {
                                        out.push(BackendEvent::ModelCreated(n.to_string()));
                                    }
                                    true
                                }
                                MODEL_DELETED => {
                                    if let Some(n) = args.first().and_then(|p| p.as_str()) {
                                        out.push(BackendEvent::ModelDeleted(n.to_string()));
                                    }
                                    true
                                }
                                RUN_START => {
                                    let time = args.first().and_then(|p| p.as_f64()).unwrap_or(0.0);
                                    out.push(BackendEvent::RunStarted { model, time });
                                    true
                                }
                                RUN_STOP => {
                                    let time = args.first().and_then(|p| p.as_f64()).unwrap_or(0.0);
                                    out.push(BackendEvent::RunStopped {
                                        model,
                                        time,
                                        events: 0,
                                    });
                                    true
                                }
                                STEP_NEXT => {
                                    out.push(BackendEvent::StepperPause {
                                        model,
                                        command: name.to_string(),
                                        args,
                                    });
                                    true
                                }
                                STEP_UPDATE => {
                                    out.push(BackendEvent::StepperUpdate {
                                        model,
                                        command: name.to_string(),
                                        args,
                                    });
                                    true
                                }
                                VV_HANDLER => {
                                    // `[name, model, verb, …]`; every verb except
                                    // `check` is a display op for the UI thread.
                                    let op: Vec<Json> = args.clone();
                                    let check = op
                                        .first()
                                        .and_then(|x| x.as_str())
                                        .map(|s| s == "check")
                                        .unwrap_or(false);
                                    if !check {
                                        out.push(BackendEvent::WindowOp(op));
                                    }
                                    true
                                }
                                _ => false,
                            }
                        }
                    };
                    let reply = if handled {
                        reply_ok(id.as_ref(), Json::Arr(vec![Json::Bool(true)]))
                    } else {
                        reply_err(id.as_ref(), &format!("no handler for {name:?}"))
                    };
                    let mut w = writer.lock().unwrap();
                    let _ = write_message(&mut *w, &reply);
                }
            }
        } else if let Some(Json::Int(id)) = msg.get("id").cloned() {
            let tx = pending.lock().unwrap().remove(&id);
            if let Some(tx) = tx {
                let _ = tx.send(parse_reply(&msg));
            }
        }
    });
}
