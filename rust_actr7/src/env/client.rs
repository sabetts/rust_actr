//! RPC-client backend: the GUI is a real ACT-R dispatcher client.
//!
//! Speaks the same `\x04`-framed JSON protocol as `tutorial/python/actr.py`
//! and the Tcl environment: a background reader thread matches reply `id`s to
//! pending requests and services server→client `evaluate` callbacks (the
//! trace monitors we register).

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

use super::backend::EnvBackend;

/// The command name the engine calls back with each trace line.
const TRACE_HANDLER: &str = "env-model-trace";
/// The four trace streams (`framework/printing.lisp`).
const TRACE_STREAMS: [&str; 4] = [
    "model-trace",
    "command-trace",
    "warning-trace",
    "general-trace",
];

type Pending = Arc<Mutex<HashMap<i64, Sender<Result<Vec<Json>, String>>>>>;

pub struct Client {
    writer: Arc<Mutex<TcpStream>>,
    pending: Pending,
    trace: Arc<Mutex<Vec<String>>>,
    next_id: AtomicI64,
    /// Cached `current-model` (queried on demand; the trait getter is `&self`).
    model: Option<String>,
    addr: String,
}

impl Client {
    /// Connect to a running `actr server` and register the trace monitors.
    pub fn connect(host: &str, port: u16) -> Result<Client, String> {
        let stream = TcpStream::connect((host, port))
            .map_err(|e| format!("could not connect to {host}:{port}: {e}"))?;
        let read_stream = stream.try_clone().map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(stream));
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));
        let trace = Arc::new(Mutex::new(Vec::new()));

        spawn_reader(
            BufReader::new(read_stream),
            writer.clone(),
            pending.clone(),
            trace.clone(),
        );

        let mut client = Client {
            writer,
            pending,
            trace,
            next_id: AtomicI64::new(1),
            model: None,
            addr: format!("{host}:{port}"),
        };

        client.request("set-name", vec![Json::Str("ACT-R Environment".into())])?;
        // Register the trace sink as a client-owned command, then monitor all
        // four streams onto it (as `11-ctrl-panel-load-button.tcl` does).
        client.request(
            "add",
            vec![
                Json::Str(TRACE_HANDLER.into()),
                Json::Str(TRACE_HANDLER.into()),
                Json::Str("Environment trace sink".into()),
                Json::Null,
                Json::Str(TRACE_HANDLER.into()),
            ],
        )?;
        for stream in TRACE_STREAMS {
            client.request(
                "monitor",
                vec![
                    Json::Str(stream.into()),
                    Json::Str(TRACE_HANDLER.into()),
                    Json::Str("simple".into()),
                ],
            )?;
        }
        let _ = client.refresh_model();
        Ok(client)
    }

    /// Send a request and block for its reply.
    fn request(&self, method: &str, params: Vec<Json>) -> Result<Vec<Json>, String> {
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
        match rx.recv_timeout(Duration::from_secs(600)) {
            Ok(r) => r,
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
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
            if let Some(Json::Str(name)) = v.first() {
                self.model = Some(name.clone());
            }
        }
        self.model.clone()
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
            "load-act-r-model" | "load-act-r-code" | "reset" | "set-current-model"
        ) {
            let _ = self.refresh_model();
        }
        result
    }

    fn take_trace(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.trace.lock().unwrap())
    }

    fn current_model(&self) -> Option<String> {
        self.model.clone()
    }

    fn set_current_model(&mut self, name: Option<String>) {
        let arg = name.clone().map(Json::Str).unwrap_or(Json::Null);
        let _ = self.evaluate("set-current-model", None, vec![arg]);
        self.model = name;
    }

    fn describe(&self) -> String {
        format!("RPC client → {}", self.addr)
    }
}

/// The reader loop: replies to pending requests, and trace callbacks to the
/// shared trace queue.
fn spawn_reader(
    mut reader: BufReader<TcpStream>,
    writer: Arc<Mutex<TcpStream>>,
    pending: Pending,
    trace: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || loop {
        let msg = match read_message(&mut reader) {
            Ok(Some(m)) => m,
            _ => break,
        };
        if msg.get("method").is_some() {
            // A server→client callback (a monitor we registered).
            if let Ok((method, params, id)) = parse_request(&msg) {
                if method == Method::Evaluate {
                    let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
                    let reply = if name == TRACE_HANDLER {
                        if let Some(line) = params.get(2).and_then(|p| p.as_str()) {
                            trace.lock().unwrap().push(line.to_string());
                        }
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
