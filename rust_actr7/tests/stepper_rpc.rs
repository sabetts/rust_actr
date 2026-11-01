//! Self-contained stepper RPC test: a Rust client drives the Rust server over
//! the real socket protocol. It uses only the purpose-built stepper fixture
//! model under `fixtures/stepper/` — no Python, SBCL, or ACT-R checkout.

use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use actr_engine::json::Json;
use actr_engine::rpc::server::serve_listener;
use actr_engine::rpc::{parse_reply, read_message, reply_ok, write_message};

struct Client {
    stream: TcpStream,
    next_id: i64,
}

impl Client {
    fn connect(addr: std::net::SocketAddr) -> io::Result<Client> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
        Ok(Client { stream, next_id: 1 })
    }

    fn send(&mut self, method: &str, params: Vec<Json>) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let msg = Json::Obj(vec![
            ("method".into(), Json::Str(method.into())),
            ("params".into(), Json::Arr(params)),
            ("id".into(), Json::Int(id)),
        ]);
        write_message(&mut self.stream, &msg).expect("send request");
        id
    }

    fn read(&mut self) -> Json {
        read_message(&mut self.stream)
            .expect("read message")
            .expect("connection stayed open")
    }

    /// Reply to a server→client callback with a successful result.
    fn reply_callback(&mut self, msg: &Json) {
        let id = msg.get("id").cloned().unwrap_or(Json::Null);
        let reply = reply_ok(Some(&id), Json::Arr(vec![Json::Bool(true)]));
        write_message(&mut self.stream, &reply).expect("send callback reply");
    }

    fn message_id(msg: &Json) -> Option<i64> {
        match msg.get("id") {
            Some(Json::Int(i)) => Some(*i),
            _ => None,
        }
    }

    /// Send an `evaluate` request and wait for its reply, answering any
    /// incidental callbacks with success first.
    fn eval(&mut self, name: &str, model: Option<&str>, args: Vec<Json>) -> Vec<Json> {
        let mut params = vec![
            Json::Str(name.into()),
            model.map(|m| Json::Str(m.into())).unwrap_or(Json::Null),
        ];
        params.extend(args);
        let id = self.send("evaluate", params);
        loop {
            let msg = self.read();
            if msg.get("method").is_some() {
                self.reply_callback(&msg);
                continue;
            }
            if Self::message_id(&msg) == Some(id) {
                return parse_reply(&msg).expect("successful reply");
            }
        }
    }

    fn add_command(&mut self, name: &str, proc_name: &str) {
        // Tcl's four-argument `add` form: name, client proc, doc, single.
        let id = self.send(
            "add",
            vec![
                Json::Str(name.into()),
                Json::Str(proc_name.into()),
                Json::Str("stepper test callback".into()),
                Json::Null,
            ],
        );
        loop {
            let msg = self.read();
            if Self::message_id(&msg) == Some(id) {
                break;
            }
        }
    }
}

fn stepper_model() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("stepper")
        .join("stepper-model.lisp")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn rpc_stepper_pauses_updates_and_resumes() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("listener address");
    std::thread::spawn(move || {
        let _ = serve_listener(listener, (100, 0));
    });

    let mut client = Client::connect(addr).expect("connect test server");
    client.add_command("next-event", "next-event");
    client.add_command("update-event", "update-event");

    let loaded = client.eval("load-act-r-model", None, vec![Json::Str(stepper_model())]);
    assert_eq!(loaded.first(), Some(&Json::Bool(true)));

    let started = client.eval(
        "start-stepper",
        None,
        vec![
            Json::Bool(false),
            Json::Bool(false),
            Json::Str("next-event".into()),
            Json::Str("update-event".into()),
        ],
    );
    assert_eq!(started.first(), Some(&Json::Bool(true)), "{started:?}");
    let stepper_id = started[1].as_str().expect("stepper id").to_string();
    let ready = client.eval(
        "set-stepper-ready",
        None,
        vec![Json::Str(stepper_id.clone())],
    );
    assert_eq!(ready.first(), Some(&Json::Bool(true)));

    let run_id = client.send(
        "evaluate",
        vec![Json::Str("run".into()), Json::Null, Json::Float(2.0)],
    );

    let mut next_events = Vec::new();
    let mut updates = Vec::new();
    let mut steps = 0;
    let run_result = loop {
        let msg = client.read();
        if msg.get("method").is_some() {
            let params = match msg.get("params") {
                Some(Json::Arr(a)) => a.clone(),
                _ => Vec::new(),
            };
            let callback = params.first().and_then(|p| p.as_str()).unwrap_or("");
            match callback {
                "next-event" => {
                    let desc = params.get(2).and_then(|p| p.as_str()).unwrap_or("");
                    next_events.push(desc.to_string());
                    client.reply_callback(&msg);
                    if steps < 3 {
                        let reply =
                            client.eval("step-stepper", None, vec![Json::Str(stepper_id.clone())]);
                        assert_eq!(reply.first(), Some(&Json::Bool(true)), "{reply:?}");
                        steps += 1;
                    } else {
                        let reply = client.eval(
                            "step-stepper",
                            None,
                            vec![Json::Str(stepper_id.clone()), Json::Bool(true)],
                        );
                        assert_eq!(reply.first(), Some(&Json::Bool(true)), "{reply:?}");
                    }
                }
                "update-event" => {
                    assert_eq!(params.len(), 9, "update callback shape: {params:?}");
                    updates.push(
                        params
                            .get(2)
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string(),
                    );
                    client.reply_callback(&msg);
                }
                _ => client.reply_callback(&msg),
            }
            continue;
        }
        if Client::message_id(&msg) == Some(run_id) {
            break parse_reply(&msg).expect("run reply");
        }
    };
    assert_eq!(run_result.len(), 3, "run returns (time events break)");
    assert!(
        next_events
            .first()
            .is_some_and(|e| e.contains("SET-BUFFER-CHUNK GOAL")),
        "first paused event: {next_events:?}"
    );
    assert!(
        next_events
            .iter()
            .any(|e| e.contains("CONFLICT-RESOLUTION")),
        "expected conflict resolution in {next_events:?}"
    );
    assert!(!updates.is_empty(), "expected update callbacks");
    assert_eq!(
        updates.len(),
        next_events.len().min(updates.len()),
        "updates correspond to stepped events"
    );
    assert!(
        updates
            .first()
            .is_some_and(|e| e.contains("SET-BUFFER-CHUNK GOAL")),
        "first update: {updates:?}"
    );

    let stopped = client.eval("stop-stepper", None, vec![Json::Str(stepper_id)]);
    assert_eq!(stopped.first(), Some(&Json::Bool(true)));
}

#[test]
fn rpc_load_notifies_the_current_model_menu_monitor() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("listener address");
    std::thread::spawn(move || {
        let _ = serve_listener(listener, (100, 0));
    });

    let mut client = Client::connect(addr).expect("connect test server");
    // Tcl's procedure name differs from the command name: the server must call
    // the procedure from `add` params[1], not the local alias.
    client.add_command("current-add", "create-current-model");
    let monitor_id = client.send(
        "monitor",
        vec![
            Json::Str("creating-model".into()),
            Json::Str("current-add".into()),
            Json::Str("simple".into()),
        ],
    );
    loop {
        let msg = client.read();
        if Client::message_id(&msg) == Some(monitor_id) {
            break;
        }
    }

    let model = stepper_model();
    let load_id = client.send(
        "evaluate",
        vec![
            Json::Str("load-act-r-model".into()),
            Json::Null,
            Json::Str(model),
        ],
    );
    let mut monitor_args = None;
    loop {
        let msg = client.read();
        if msg.get("method").is_some() {
            if let Some(Json::Arr(params)) = msg.get("params") {
                if params.first().and_then(|p| p.as_str()) == Some("create-current-model") {
                    monitor_args = Some(params.clone());
                }
            }
            client.reply_callback(&msg);
            continue;
        }
        if Client::message_id(&msg) == Some(load_id) {
            parse_reply(&msg).expect("load reply");
            break;
        }
    }
    let params = monitor_args.expect("creating-model monitor callback");
    assert_eq!(params.get(1), Some(&Json::Str("STEPPER-FIXTURE".into())));
    assert_eq!(params.get(2), Some(&Json::Str("STEPPER-FIXTURE".into())));

    let models = client.eval("mp-models", None, Vec::new());
    assert_eq!(
        models,
        vec![Json::Arr(vec![Json::Str("STEPPER-FIXTURE".into())])]
    );
}
