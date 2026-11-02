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
        stream.set_read_timeout(Some(Duration::from_secs(20)))?;
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

    fn set_name(&mut self, name: &str) {
        let id = self.send(Method::SetName, vec![Json::Str(name.into())]);
        self.wait(id).expect("set-name");
    }

    fn add_command(&mut self, name: &str, local: &str) {
        let id = self.send(
            Method::Add,
            vec![Json::Str(name.into()), Json::Str(local.into())],
        );
        self.wait(id).expect("add command");
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
        && args.first().and_then(|a| a.as_str()) == Some("cmd line")));
    assert!(c.callbacks.iter().any(|(n, _, args)| n == "on-general"
        && args.first().and_then(|a| a.as_str()) == Some("general line")));

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
fn reload_reads_the_file_from_disk_and_multi_model_is_rejected() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
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

    let edited = std::fs::read_to_string(&path)
        .unwrap()
        .replace("(define-model count2", "(define-model count3");
    std::fs::write(&path, edited).unwrap();
    c.eval_ok("reload", None, Vec::new());
    assert_eq!(
        strings(&c.eval_ok("mp-models", None, Vec::new())),
        vec!["COUNT3"]
    );

    let err = c
        .eval(
            "load-act-r-model",
            None,
            vec![Json::Str(model("tutorial/unit1/addition.lisp"))],
        )
        .unwrap_err();
    assert!(err.contains("Multiple models are not supported"), "{err}");
    let _ = std::fs::remove_file(path);
}

#[test]
fn viewers_connections_and_history_round_trip() {
    let addr = server();
    let mut c = Client::connect(addr).expect("connect");
    c.set_name("ACT-R Environment");
    c.add_command("env-probe", "env-probe");
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
