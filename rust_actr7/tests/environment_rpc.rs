//! Environment-shaped RPC tests: a fake Tcl client exercises the control
//! panel, model lifecycle, viewers, event queue, and histories over the real
//! socket protocol. No Tcl/wish installation is needed.

use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use actr_engine::json::Json;
use actr_engine::rpc::server::serve_listener;
use actr_engine::rpc::{parse_reply, read_message, reply_ok, write_message, Method};

struct Client {
    stream: TcpStream,
    next_id: i64,
    callbacks: Vec<(String, Option<String>, Vec<Json>)>,
}

impl Client {
    fn connect(addr: std::net::SocketAddr) -> io::Result<Client> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        Ok(Client {
            stream,
            next_id: 1,
            callbacks: Vec::new(),
        })
    }

    fn send(&mut self, method: Method, params: Vec<Json>) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let method = match method {
            Method::Add => "add",
            Method::Remove => "remove",
            Method::Evaluate => "evaluate",
            Method::Monitor => "monitor",
            Method::RemoveMonitor => "remove-monitor",
            Method::ListCommands => "list-commands",
            Method::ListConnections => "list-connections",
            Method::Check => "check",
            Method::SetName => "set-name",
            Method::Close => "close",
        };
        let msg = Json::Obj(vec![
            ("method".into(), Json::Str(method.into())),
            ("params".into(), Json::Arr(params)),
            ("id".into(), Json::Int(id)),
        ]);
        write_message(&mut self.stream, &msg).expect("send request");
        id
    }

    fn wait(&mut self, id: i64) -> Result<Vec<Json>, String> {
        loop {
            let msg = read_message(&mut self.stream)
                .expect("read message")
                .expect("connection stayed open");
            if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                if method == "evaluate" {
                    let params = match msg.get("params") {
                        Some(Json::Arr(a)) => a.clone(),
                        _ => Vec::new(),
                    };
                    let name = params
                        .first()
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string();
                    let model = params.get(1).and_then(|p| p.as_str()).map(str::to_string);
                    self.callbacks
                        .push((name, model, params.get(2..).unwrap_or(&[]).to_vec()));
                    let rid = msg.get("id").cloned().unwrap_or(Json::Null);
                    let reply = reply_ok(Some(&rid), Json::Arr(vec![Json::Bool(true)]));
                    write_message(&mut self.stream, &reply).expect("send callback reply");
                    continue;
                }
            }
            if msg.get("id").and_then(|i| match i {
                Json::Int(n) => Some(*n),
                _ => None,
            }) == Some(id)
            {
                return parse_reply(&msg);
            }
        }
    }

    fn read(&mut self) -> Json {
        read_message(&mut self.stream)
            .expect("read message")
            .expect("connection stayed open")
    }

    fn reply_callback(&mut self, msg: &Json) {
        let id = msg.get("id").cloned().unwrap_or(Json::Null);
        let reply = reply_ok(Some(&id), Json::Arr(vec![Json::Bool(true)]));
        write_message(&mut self.stream, &reply).expect("send callback reply");
    }

    fn set_name(&mut self, name: &str) {
        let id = self.send(Method::SetName, vec![Json::Str(name.into())]);
        self.wait(id).expect("set-name");
    }

    fn add_command(&mut self, name: &str, local: &str) {
        self.add_command_doc(name, local, "");
    }

    fn add_command_doc(&mut self, name: &str, local: &str, doc: &str) {
        let id = self.send(
            Method::Add,
            vec![
                Json::Str(name.into()),
                Json::Str(local.into()),
                Json::Str(doc.into()),
            ],
        );
        self.wait(id).expect("add command");
    }

    fn check(&mut self, name: &str) -> Vec<Json> {
        let id = self.send(Method::Check, vec![Json::Str(name.into())]);
        self.wait(id).expect("check")
    }

    fn monitor(&mut self, target: &str, monitor: &str) {
        let id = self.send(
            Method::Monitor,
            vec![
                Json::Str(target.into()),
                Json::Str(monitor.into()),
                Json::Str("simple".into()),
            ],
        );
        self.wait(id).expect("monitor");
    }

    fn eval(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        let mut params = vec![
            Json::Str(name.into()),
            model.map(|m| Json::Str(m.into())).unwrap_or(Json::Null),
        ];
        params.extend(args);
        let id = self.send(Method::Evaluate, params);
        self.wait(id)
    }

    fn eval_ok(&mut self, name: &str, model: Option<&str>, args: Vec<Json>) -> Vec<Json> {
        self.eval(name, model, args)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    fn connections(&mut self) -> Vec<Json> {
        let id = self.send(Method::ListConnections, Vec::new());
        self.wait(id).expect("list-connections")
    }

    fn list_commands(&mut self) -> Vec<Json> {
        let id = self.send(Method::ListCommands, Vec::new());
        self.wait(id).expect("list-commands")
    }
}

fn model(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("models")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn server() -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("listener address");
    std::thread::spawn(move || {
        let _ = serve_listener(listener, (100, 0));
    });
    addr
}

fn strings(v: &[Json]) -> Vec<String> {
    match v.first() {
        Some(Json::Arr(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn control_panel_signals_event_queue_and_clear_all() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.add_command("on-start", "on-start");
    c.add_command("on-stop", "on-stop");
    c.add_command("on-reset", "on-reset");
    c.add_command("on-clear", "on-clear");
    c.monitor("run-start", "on-start");
    c.monitor("run-stop", "on-stop");
    c.monitor("reset-step1", "on-reset");
    c.monitor("clear-all-start", "on-clear");

    let loaded = c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    assert_eq!(loaded.first(), Some(&Json::Bool(true)));

    c.add_command("on-warning", "on-warning");
    c.monitor("warning-trace", "on-warning");
    c.eval_ok("print-warning", None, vec![Json::Str("rpc warning".into())]);
    assert!(
        c.callbacks.iter().any(|(n, _, _)| n == "on-warning"),
        "warning-trace monitor did not run: {:?}",
        c.callbacks
    );

    c.add_command("on-command", "on-command");
    c.add_command("on-general", "on-general");
    c.monitor("command-trace", "on-command");
    c.monitor("general-trace", "on-general");
    c.eval_ok("command-output", None, vec![Json::Str("cmd line".into())]);
    c.eval_ok("act-r-output", None, vec![Json::Str("general line".into())]);
    assert!(c.callbacks.iter().any(|(n, _, args)| n == "on-command"
        && args.first().and_then(|a| a.as_str()) == Some("cmd line\n")));
    assert!(c.callbacks.iter().any(|(n, _, args)| n == "on-general"
        && args.first().and_then(|a| a.as_str()) == Some("general line\n")));

    let queue = c.eval_ok("mp-queue-text", None, Vec::new());
    assert!(
        queue
            .first()
            .and_then(|x| x.as_str())
            .is_some_and(|s| s.contains("Events in the queue")),
        "{queue:?}"
    );

    c.eval_ok("run", None, vec![Json::Float(2.0)]);
    assert!(
        c.callbacks.iter().any(|(n, _, _)| n == "on-start"),
        "missing run-start: {:?}",
        c.callbacks
    );
    assert!(
        c.callbacks.iter().any(|(n, _, _)| n == "on-stop"),
        "missing run-stop: {:?}",
        c.callbacks
    );

    c.eval_ok("reset", None, Vec::new());
    assert!(
        c.callbacks.iter().any(|(n, _, _)| n == "on-reset"),
        "missing reset-step1: {:?}",
        c.callbacks
    );

    c.eval_ok("clear-all", None, Vec::new());
    assert!(
        c.callbacks.iter().any(|(n, _, _)| n == "on-clear"),
        "missing clear-all-start: {:?}",
        c.callbacks
    );
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        Vec::<String>::new()
    );
}

#[test]
fn reload_reads_the_file_from_disk_and_loads_another_model() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    let path = std::env::temp_dir().join(format!(
        "actr-reload-{}-{}.lisp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let source = std::fs::read_to_string(model("tutorial/unit1/count.lisp")).unwrap();
    std::fs::write(
        &path,
        source.replace("(define-model count", "(define-model count2"),
    )
    .unwrap();
    let file = path.to_string_lossy().into_owned();

    c.eval_ok("load-act-r-model", None, vec![Json::Str(file.clone())]);
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        vec!["COUNT2"]
    );
    c.eval_ok(
        "open-exp-window",
        None,
        vec![Json::Str("COUNT2-WIN".into()), Json::Null],
    );

    let edited = std::fs::read_to_string(&path)
        .unwrap()
        .replace("(define-model count2", "(define-model count3");
    std::fs::write(&path, edited).unwrap();
    c.eval_ok("reload", None, Vec::new());
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        vec!["COUNT3"]
    );
    let closes = window_ops(&c, "close");
    assert!(closes.iter().any(|(model, args)| {
        model.as_deref() == Some("COUNT2")
            && args.get(1).and_then(|x| x.as_str()) == Some("COUNT2-WIN")
    }));

    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        vec!["COUNT3", "ADDITION"]
    );
    assert_eq!(
        c.eval_ok("current-model", None, Vec::new()),
        vec![Json::Str("ADDITION".into())]
    );
    c.eval_ok("set-current-model", None, vec![Json::Str("COUNT3".into())]);
    assert_eq!(
        c.eval_ok("current-model", None, Vec::new()),
        vec![Json::Str("COUNT3".into())]
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn stop_request_interrupts_a_running_request() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("test-models/run-control.lisp"))],
    );

    // Send the run, then Stop on the same connection. The server must pump
    // the Stop request between run slices instead of blocking on `run`.
    let run_id = c.send(
        Method::Evaluate,
        vec![Json::Str("run".into()), Json::Null, Json::Float(60.0)],
    );
    let stop_id = c.send(
        Method::Evaluate,
        vec![
            Json::Str("schedule-break-relative".into()),
            Json::Null,
            Json::Int(0),
        ],
    );
    c.wait(stop_id).expect("schedule-break-relative");
    let run = c.wait(run_id).expect("run");
    let time = run.first().and_then(|x| x.as_f64()).unwrap_or(60.0);
    assert!(
        time < 60.0,
        "run should stop early after schedule-break-relative, got {time}"
    );
}

#[test]
fn connections_share_one_model_session() {
    let addr = server();
    let mut python = Client::connect(addr).expect("connect python");
    let mut env = Client::connect(addr).expect("connect environment");
    python.set_name("Python task");
    env.set_name("ACT-R Environment");

    python.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    assert_eq!(
        strings(&env.eval_ok("mp-models", None, Vec::new())),
        vec!["COUNT".to_string()]
    );
    assert!(strings(&env.eval_ok("all-productions", None, Vec::new()))
        .iter()
        .any(|p| p.contains("START")));

    let conns = env.connections();
    let Some(Json::Arr(rows)) = conns.first() else {
        panic!("connections is not an array: {conns:?}");
    };
    let names: Vec<&str> = rows
        .iter()
        .filter_map(|row| match row {
            Json::Arr(fields) => fields.first().and_then(Json::as_str),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"Python task"), "{names:?}");
    assert!(names.contains(&"ACT-R Environment"), "{names:?}");
    assert!(names.contains(&"internal ACT-R commands"), "{names:?}");
}

#[test]
fn one_connection_can_stop_another_connection_run() {
    let addr = server();
    let mut runner = Client::connect(addr).expect("connect runner");
    let mut env = Client::connect(addr).expect("connect environment");
    runner.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("test-models/run-control.lisp"))],
    );

    let run_id = runner.send(
        Method::Evaluate,
        vec![Json::Str("run".into()), Json::Null, Json::Float(60.0)],
    );
    let stop_id = env.send(
        Method::Evaluate,
        vec![
            Json::Str("schedule-break-relative".into()),
            Json::Null,
            Json::Int(0),
        ],
    );
    env.wait(stop_id).expect("schedule-break-relative");
    let run = runner.wait(run_id).expect("run");
    let time = run.first().and_then(|x| x.as_f64()).unwrap_or(60.0);
    assert!(
        time < 60.0,
        "shared run should stop early after another connection breaks it, got {time}"
    );
}

#[test]
fn trace_monitor_services_requests_on_the_owning_connection() {
    let addr = server();
    let mut python = Client::connect(addr).expect("connect python");
    let mut env = Client::connect(addr).expect("connect environment");
    env.add_command("env-trace", "env-trace");
    env.monitor("model-trace", "env-trace");

    let python = std::thread::spawn(move || {
        python.eval_ok(
            "load-act-r-model",
            None,
            vec![Json::Str(model("tutorial/unit1/count.lisp"))],
        );
        python.eval_ok("run", None, vec![Json::Float(2.0)]);
        python
    });

    for _ in 0..1000 {
        // Keep reading this connection so a trace callback can be answered
        // while the other connection is still inside `run`.
        let id = env.send(
            Method::Evaluate,
            vec![Json::Str("get-time".into()), Json::Null],
        );
        env.wait(id).expect("get-time");
        if python.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let _ = python.join().expect("python task");
    assert!(
        env.callbacks.iter().any(|(name, _, _)| name == "env-trace"),
        "trace monitor was not called: {:?}",
        env.callbacks
    );
}

#[test]
fn disconnecting_removes_owned_commands_and_monitors() {
    let addr = server();
    let mut owner = Client::connect(addr).expect("connect owner");
    let mut env = Client::connect(addr).expect("connect environment");
    owner.set_name("owner");
    env.set_name("ACT-R Environment");
    owner.add_command("gone-command", "gone-command");
    owner.monitor("model-trace", "gone-command");

    drop(owner);
    for _ in 0..100 {
        let rows = env.connections();
        let Some(Json::Arr(rows)) = rows.first() else {
            panic!("connections is not an array: {rows:?}");
        };
        let owner_gone = !rows.iter().any(|row| {
            matches!(row, Json::Arr(fields)
                if fields.first().and_then(Json::as_str) == Some("owner"))
        });
        if owner_gone {
            let listed = env.list_commands();
            let Some(Json::Arr(items)) = listed.first() else {
                panic!("list-commands is not an array: {listed:?}");
            };
            assert!(
                !items.iter().any(|item| {
                    matches!(item, Json::Obj(fields)
                        if fields.iter().any(|(k, v)| k == "name"
                            && v.as_str() == Some("gone-command")))
                }),
                "disconnected owner's command is still registered"
            );
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("owner connection was not removed");
}

#[test]
fn multi_model_run_emits_both_model_columns() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.add_command("on-trace", "on-trace");
    c.monitor("model-trace", "on-trace");
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    c.eval_ok("run", None, vec![Json::Float(2.0)]);
    let lines: Vec<&str> = c
        .callbacks
        .iter()
        .filter(|(n, _, _)| n == "on-trace")
        .filter_map(|(_, _, args)| args.first().and_then(|a| a.as_str()))
        .collect();
    assert!(lines.iter().any(|l| l.contains("COUNT")), "{lines:?}");
    assert!(lines.iter().any(|l| l.contains("ADDITION")), "{lines:?}");
    let stops: Vec<_> = lines
        .iter()
        .filter(|l| l.contains("Stopped because"))
        .collect();
    assert_eq!(stops.len(), 1, "{lines:?}");
    assert!(stops[0].contains(" - "), "{lines:?}");
    c.eval_ok("clear-all", None, Vec::new());
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        Vec::<String>::new()
    );
}

#[test]
fn viewers_connections_and_history_round_trip() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.set_name("ACT-R Environment");
    c.add_command_doc("env-probe", "env-probe", "probe documentation");
    let checked = c.check("env-probe");
    assert_eq!(
        checked.get(2).and_then(|v| v.as_str()),
        Some("probe documentation")
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok("run", None, vec![Json::Float(2.0)]);

    let prods = strings(&c.eval_ok("all-productions", None, Vec::new()));
    assert!(!prods.is_empty());
    let chunks = strings(&c.eval_ok("filter-dm-chunks", None, vec![Json::Str("_none_".into())]));
    assert!(!chunks.is_empty());
    let detail = c.eval_ok("dm-chunk-details", None, vec![Json::Str(chunks[0].clone())]);
    assert!(detail[0]
        .as_str()
        .is_some_and(|s| s.contains("Declarative parameters")));
    let prod = c.eval_ok(
        "production-details",
        None,
        vec![Json::Str(prods[0].clone())],
    );
    assert!(prod[0]
        .as_str()
        .is_some_and(|s| s.contains("Parameters for production")));
    let params = strings(&c.eval_ok("sorted-module-names", None, Vec::new()));
    assert!(params.iter().any(|m| m == "BLENDING"));
    let pdetail = c.eval_ok(
        "printed-parameter-details",
        None,
        vec![Json::Str(":RT".into())],
    );
    assert!(pdetail[0].as_str().is_some_and(|s| s.starts_with(":RT ")));

    let conns = c.connections();
    let Some(Json::Arr(entries)) = conns.first() else {
        panic!("connections is not an array: {conns:?}");
    };
    let Some(Json::Arr(first)) = entries.first() else {
        panic!("no connection entry: {conns:?}");
    };
    assert_eq!(first[0].as_str(), Some("ACT-R Environment"));
    assert!(
        first[2].as_f64().is_some_and(|v| v > 0.0),
        "received counter: {first:?}"
    );
    assert!(
        first[3].as_f64().is_some_and(|v| v > 0.0),
        "sent counter: {first:?}"
    );
    let Some(Json::Arr(cmds)) = first.get(4) else {
        panic!("connection commands missing: {first:?}");
    };
    assert!(cmds.iter().any(|n| n.as_str() == Some("env-probe")));

    c.eval_ok(
        "record-history",
        None,
        vec![Json::Str("trace-history".into())],
    );
    c.eval_ok("run", None, vec![Json::Float(2.0)]);
    let available = c.eval_ok(
        "history-data-available",
        None,
        vec![Json::Str("trace-history".into())],
    );
    assert_eq!(available.first(), Some(&Json::Bool(true)));
    let id = c.eval_ok(
        "start-incremental-history-data",
        None,
        vec![Json::Str("trace-history".into()), Json::Int(16000)],
    );
    let mut chunks = Vec::new();
    loop {
        let got = c.eval_ok("get-incremental-history-data", None, id.clone());
        match got.first().and_then(|x| x.as_str()) {
            Some("") | None => break,
            Some(s) => chunks.push(s.to_string()),
        }
    }
    assert!(!chunks.is_empty(), "trace-history produced no chunks");
}
#[test]
fn multi_model_stepper_callbacks_include_the_model() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.add_command("next-event", "next-event");
    c.add_command("update-event", "update-event");
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    let started = c
        .eval(
            "start-stepper",
            None,
            vec![
                Json::Bool(false),
                Json::Bool(false),
                Json::Str("next-event".into()),
                Json::Str("update-event".into()),
            ],
        )
        .expect("start stepper");
    let stepper_id = started[1].as_str().expect("stepper id").to_string();
    c.eval_ok(
        "set-stepper-ready",
        None,
        vec![Json::Str(stepper_id.clone())],
    );

    let run_id = c.send(
        Method::Evaluate,
        vec![Json::Str("run".into()), Json::Null, Json::Float(2.0)],
    );
    let mut paused_models = Vec::new();
    let mut updates = 0;
    let mut steps = 0;
    loop {
        let msg = c.read();
        if msg.get("method").is_some() {
            let params = match msg.get("params") {
                Some(Json::Arr(a)) => a.clone(),
                _ => Vec::new(),
            };
            let callback = params.first().and_then(|p| p.as_str()).unwrap_or("");
            let model = params.get(1).and_then(|p| p.as_str()).map(str::to_string);
            c.reply_callback(&msg);
            match callback {
                "next-event" => {
                    paused_models.push(model);
                    if steps < 3 {
                        c.eval_ok("step-stepper", None, vec![Json::Str(stepper_id.clone())]);
                        steps += 1;
                    } else {
                        c.eval_ok(
                            "step-stepper",
                            None,
                            vec![Json::Str(stepper_id.clone()), Json::Bool(true)],
                        );
                    }
                }
                "update-event" => updates += 1,
                _ => {}
            }
            continue;
        }
        let rid = msg.get("id").and_then(|i| match i {
            Json::Int(n) => Some(*n),
            _ => None,
        });
        if rid == Some(run_id) {
            parse_reply(&msg).expect("run reply");
            break;
        }
    }
    assert!(!paused_models.is_empty(), "no stepper pauses");
    assert!(
        paused_models
            .iter()
            .flatten()
            .all(|m| m == "COUNT" || m == "ADDITION"),
        "{paused_models:?}"
    );
    assert!(updates > 0, "no stepper updates");
}

fn options(pairs: Vec<(&str, Json)>) -> Json {
    Json::Arr(
        pairs
            .into_iter()
            .map(|(k, v)| Json::Arr(vec![Json::Str(k.into()), v]))
            .collect(),
    )
}

fn window_ops(c: &Client, op: &str) -> Vec<(Option<String>, Vec<Json>)> {
    c.callbacks
        .iter()
        .filter(|(name, _, _)| name == "env-vv")
        .filter_map(|(_, model, args)| {
            (args.first().and_then(|a| a.as_str()) == Some(op))
                .then(|| (model.clone(), args.clone()))
        })
        .collect()
}

fn add_window_handler(c: &mut Client) {
    c.add_command("env-vv", "env-vv");
    c.eval_ok(
        "add-virtual-window-handler",
        None,
        vec![Json::Str("env-vv".into())],
    );
}

#[test]
fn multi_model_windows_are_scoped_by_model() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );

    c.eval_ok(
        "open-exp-window",
        Some("COUNT"),
        vec![Json::Str("COUNT-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "open-exp-window",
        Some("ADDITION"),
        vec![Json::Str("ADDITION-WIN".into()), Json::Null],
    );

    let count_key = c
        .eval_ok(
            "add-text-to-exp-window",
            Some("COUNT"),
            vec![Json::Str("COUNT-WIN".into()), Json::Str("A".into())],
        )
        .first()
        .and_then(|j| j.as_str())
        .expect("count text key")
        .to_string();
    let addition_key = c
        .eval_ok(
            "add-text-to-exp-window",
            Some("ADDITION"),
            vec![Json::Str("ADDITION-WIN".into()), Json::Str("B".into())],
        )
        .first()
        .and_then(|j| j.as_str())
        .expect("addition text key")
        .to_string();

    let opens = window_ops(&c, "open");
    assert!(opens.iter().any(|(m, a)| m.as_deref() == Some("COUNT")
        && a.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));
    assert!(opens.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")));

    let texts = window_ops(&c, "text");
    assert!(texts.iter().any(|(m, a)| m.as_deref() == Some("COUNT")
        && a.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")
        && a.get(2).and_then(|x| x.as_str()) == Some(count_key.as_str())));
    assert!(texts.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")
        && a.get(2).and_then(|x| x.as_str()) == Some(addition_key.as_str())));

    c.eval_ok(
        "close-exp-window",
        Some("COUNT"),
        vec![Json::Str("COUNT-WIN".into())],
    );
    let closes = window_ops(&c, "close");
    assert!(closes.iter().any(|(m, a)| m.as_deref() == Some("COUNT")
        && a.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));
    assert!(!closes.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")));

    c.eval_ok(
        "add-text-to-exp-window",
        Some("ADDITION"),
        vec![Json::Str("ADDITION-WIN".into()), Json::Str("C".into())],
    );
    let texts = window_ops(&c, "text");
    assert!(texts.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")));

    c.eval_ok(
        "close-exp-window",
        Some("ADDITION"),
        vec![Json::Str("ADDITION-WIN".into())],
    );
}

#[test]
fn close_window_by_title_and_close_all_without_a_model_argument() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    c.eval_ok(
        "open-exp-window",
        Some("COUNT"),
        vec![Json::Str("COUNT-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "open-exp-window",
        Some("ADDITION"),
        vec![Json::Str("ADDITION-WIN".into()), Json::Null],
    );

    // The Environment sends the title with no model argument. Close the
    // non-current model's window by title.
    c.eval_ok(
        "close-exp-window",
        None,
        vec![Json::Str("COUNT-WIN".into())],
    );
    let closes = window_ops(&c, "close");
    assert!(closes.iter().any(|(m, a)| m.as_deref() == Some("COUNT")
        && a.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));
    assert!(!closes.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")));

    c.eval_ok("close-all-exp-windows", None, Vec::new());
    let closes = window_ops(&c, "close");
    assert!(closes.iter().any(|(m, a)| m.as_deref() == Some("ADDITION")
        && a.get(1).and_then(|x| x.as_str()) == Some("ADDITION-WIN")));
}

#[test]
fn multi_model_clear_all_closes_each_model_window() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    c.eval_ok(
        "open-exp-window",
        Some("COUNT"),
        vec![Json::Str("COUNT-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "open-exp-window",
        Some("ADDITION"),
        vec![Json::Str("ADDITION-WIN".into()), Json::Null],
    );

    c.eval_ok("clear-all", None, Vec::new());
    let closes = window_ops(&c, "close");
    assert!(closes.iter().any(|(m, _)| m.as_deref() == Some("COUNT")));
    assert!(closes.iter().any(|(m, _)| m.as_deref() == Some("ADDITION")));
}

#[test]
fn multi_model_window_click_dispatches_the_owning_models_action() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.add_command("clicked", "clicked");
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
    );
    c.eval_ok(
        "open-exp-window",
        Some("COUNT"),
        vec![Json::Str("COUNT-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "add-button-to-exp-window",
        Some("COUNT"),
        vec![
            Json::Str("COUNT-WIN".into()),
            options(vec![
                ("text", Json::Str("A".into())),
                ("x", Json::Int(0)),
                ("y", Json::Int(0)),
                ("width", Json::Int(75)),
                ("height", Json::Int(20)),
                ("action", Json::Str("clicked".into())),
            ]),
        ],
    );
    assert_eq!(
        c.eval_ok("current-model", None, Vec::new()),
        vec![Json::Str("ADDITION".into())]
    );

    c.eval_ok(
        "visible-virtual-window-mouse-click",
        None,
        vec![Json::Str("COUNT-WIN".into()), Json::Int(37), Json::Int(9)],
    );
    assert!(
        c.callbacks
            .iter()
            .any(|(name, model, _)| { name == "clicked" && model.as_deref() == Some("COUNT") }),
        "click action did not fire without a run or target COUNT: {:?}",
        c.callbacks
    );
    c.eval_ok("run", None, vec![Json::Float(2.0)]);
    assert!(
        c.callbacks
            .iter()
            .any(|(name, model, _)| { name == "clicked" && model.as_deref() == Some("COUNT") }),
        "click action did not target COUNT: {:?}",
        c.callbacks
    );
}

#[test]
fn window_button_builtin_action_dispatches_without_a_run() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "open-exp-window",
        None,
        vec![Json::Str("CLOSE-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "add-button-to-exp-window",
        None,
        vec![
            Json::Str("CLOSE-WIN".into()),
            options(vec![
                ("text", Json::Str("Close".into())),
                ("x", Json::Int(0)),
                ("y", Json::Int(0)),
                ("width", Json::Int(75)),
                ("height", Json::Int(20)),
                ("action", Json::Str("close-exp-window".into())),
            ]),
        ],
    );
    c.eval_ok(
        "visible-virtual-window-mouse-click",
        None,
        vec![Json::Str("CLOSE-WIN".into()), Json::Int(37), Json::Int(9)],
    );
    assert!(
        !window_ops(&c, "close").is_empty(),
        "button action did not close the window: {:?}",
        c.callbacks
    );
}

#[test]
fn single_model_window_uses_the_current_model() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    add_window_handler(&mut c);
    c.eval_ok(
        "load-act-r-model",
        None,
        vec![Json::Str(model("tutorial/unit1/count.lisp"))],
    );
    c.eval_ok(
        "open-exp-window",
        None,
        vec![Json::Str("COUNT-WIN".into()), Json::Null],
    );
    c.eval_ok(
        "add-text-to-exp-window",
        None,
        vec![Json::Str("COUNT-WIN".into()), Json::Str("A".into())],
    );

    assert!(window_ops(&c, "open")
        .iter()
        .any(|(model, args)| model.as_deref() == Some("COUNT")
            && args.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));
    assert!(window_ops(&c, "text")
        .iter()
        .any(|(model, args)| model.as_deref() == Some("COUNT")
            && args.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));

    c.eval_ok(
        "close-exp-window",
        None,
        vec![Json::Str("COUNT-WIN".into())],
    );
    assert!(window_ops(&c, "close")
        .iter()
        .any(|(model, args)| model.as_deref() == Some("COUNT")
            && args.get(1).and_then(|x| x.as_str()) == Some("COUNT-WIN")));
}
