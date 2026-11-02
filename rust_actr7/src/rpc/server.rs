//! The RPC listener: accept a client, read `\x04`-framed JSON requests,
//! dispatch them through the [`Session`], and stream trace output to monitors.
//!
//! Single-connection, synchronous: a client request may trigger a server→
//! client `evaluate` callback (a monitor or a client-owned command), and the
//! server blocks reading that reply before continuing — the same shape as the
//! Lisp dispatcher's `send-remote-command` (`framework/dispatcher.lisp:2074`).

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use crate::json::Json;
use crate::rpc::session::{MonitorWhen, Owner, Remote, RunStop, Session};
use crate::rpc::{
    callback, parse_reply, parse_request, read_message, reply_err, reply_ok, write_message, Method,
};

/// Serve clients on `addr` (TCP).  Model and task files are loaded from
/// filesystem paths supplied by the client.
///
/// Each connection is handled on its own thread so a second client (the
/// Python `actr.connection()` a test may open alongside the driver) is
/// answered immediately instead of blocking behind the first — the Lisp
/// server accepts clients concurrently too.
pub fn serve(addr: &str, seed: (u64, u64)) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    serve_listener(listener, seed)
}

/// Serve an already-bound listener. Tests use this with port 0 so they can
/// read the assigned address without racing a spawned process.
pub fn serve_listener(listener: TcpListener, seed: (u64, u64)) -> std::io::Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        std::thread::spawn(move || {
            let mut session = Session::new(seed);
            if let Err(e) = handle(&mut session, stream) {
                eprintln!("rpc: connection ended: {e}");
            }
        });
    }
    Ok(())
}

/// Write the ACT-R port/host files the Python client looks for
/// (`tutorial/python/actr.py:190`).
pub fn write_port_file(host: &str, port: u16) -> std::io::Result<()> {
    // `actr.py` does `int(f.readline())` for the port (newline-tolerant) but
    // uses the host line verbatim, so neither may carry a trailing newline.
    // `home_dir` matches Python's `expanduser("~")` on both Unix and Windows.
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    std::fs::write(home.join("act-r-port-num.txt"), port.to_string())?;
    std::fs::write(home.join("act-r-address.txt"), host)?;
    Ok(())
}

fn handle(session: &mut Session, stream: TcpStream) -> std::io::Result<()> {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_default();
    let mut conn = Conn {
        stream,
        next_id: 1,
        name: None,
        peer,
    };
    loop {
        let msg = match crate::rpc::read_message(&mut conn.stream)? {
            Some(m) => m,
            None => return Ok(()),
        };
        let (method, params, id) = match parse_request(&msg) {
            Ok(x) => x,
            Err(e) => {
                write_message(&mut conn.stream, &reply_err(None, &e))?;
                continue;
            }
        };
        if std::env::var("RPC_DEBUG").is_ok() {
            eprintln!(
                "rpc<< {:?} {:?} id={:?}",
                method,
                params.first().and_then(|p| p.as_str()).unwrap_or(""),
                id
            );
        }
        if method == Method::Close {
            return Ok(());
        }
        let reply = dispatch(session, &mut conn, method, &params, id.as_ref());
        drain_pending_signals(session, &mut conn)?;
        // A `reset` defers a client-owned `:testing-function` to here
        // (`model-tester.cl:485`); run it with the session borrow released.
        if let Some(name) = session.take_pending_testing_fn() {
            call_remote_reentrant(session, &mut conn, &name, None, Vec::new())?;
        }
        write_message(&mut conn.stream, &reply)?;
    }
}

/// Invoke a client-owned command and service any requests the client makes
/// from inside it.
///
/// [`Conn::call`] cannot do this: it runs while `Session::evaluate` holds
/// `&mut Session`, so a nested request from the callback has no way to reach
/// the session and is dropped — leaving the client blocked forever waiting for
/// a reply. Here the borrow is released, so nested requests (`paired-learning`'s
/// testing function calls `set-parameter-value`) are dispatched normally.
fn call_remote_reentrant(
    session: &mut Session,
    conn: &mut Conn,
    name: &str,
    model: Option<&str>,
    args: Vec<Json>,
) -> std::io::Result<()> {
    let id = conn.next_id;
    conn.next_id += 1;
    let msg = callback(name, model, args, id);
    write_message(&mut conn.stream, &msg)?;
    loop {
        let reply = match read_message(&mut conn.stream)? {
            Some(m) => m,
            None => return Ok(()),
        };
        let rid = reply.get("id").and_then(|i| match i {
            Json::Int(n) => Some(*n),
            _ => None,
        });
        if rid == Some(id) {
            // The callback returned; its reply carries nothing we need.
            return Ok(());
        }
        // A request the client made from inside the callback: dispatch it.
        if let Ok((method, params, nid)) = parse_request(&reply) {
            let out = dispatch(session, conn, method, &params, nid.as_ref());
            write_message(&mut conn.stream, &out)?;
            drain_pending_signals(session, conn)?;
        }
    }
}

/// Drain the trace and the model-fired commands (`output-key`, …) to their
/// monitors. ACT-R fires those inside the run, so a task reads its handler's
/// effect as soon as `run` returns; a handler can in turn emit trace, so drain
/// until both are quiet.
fn drain(session: &mut Session, conn: &mut Conn) {
    for _ in 0..32 {
        let lines = session.take_trace();
        for line in lines {
            session.deliver_trace(&line, conn);
        }
        let fired = session.take_fired_commands();
        if fired.is_empty() {
            break;
        }
        for (cmd, model_name, key) in fired {
            let a = vec![Json::Str(model_name.clone()), Json::Str(key)];
            for (monitor, remote) in session.simple_monitors(&cmd) {
                if remote {
                    let _ = call_remote_reentrant(
                        session,
                        conn,
                        &monitor,
                        Some(&model_name),
                        a.clone(),
                    );
                } else {
                    let _ = session.evaluate(&monitor, Some(&model_name), &a, conn);
                }
            }
        }
    }
}

/// Deliver a signal (`run-start`, `reset-step1`, ...) to its simple monitors.
fn fire_signal(
    session: &mut Session,
    conn: &mut Conn,
    name: &str,
    model: Option<&str>,
    args: Vec<Json>,
) -> std::io::Result<()> {
    for (monitor, remote) in session.simple_monitors(name) {
        if remote {
            call_remote_reentrant(session, conn, &monitor, model, args.clone())?;
        } else {
            let _ = session.evaluate(&monitor, model, &args, conn);
        }
    }
    drain(session, conn);
    Ok(())
}

fn drain_pending_signals(session: &mut Session, conn: &mut Conn) -> std::io::Result<()> {
    for (name, model, args) in session.take_pending_signals() {
        fire_signal(session, conn, &name, model.as_deref(), args)?;
    }
    Ok(())
}

/// Deliver stepper update callbacks until none are pending.
fn drain_stepper_updates(session: &mut Session, conn: &mut Conn) -> std::io::Result<()> {
    while let Some((cmd, args)) = session.take_stepper_update() {
        call_remote_reentrant(session, conn, &cmd, None, args)?;
        drain(session, conn);
    }
    Ok(())
}

/// While a stepper is paused before an event, keep servicing client requests
/// (Step / Stop / Update / Set-*) without answering the pending `run` request.
fn pump_stepper(session: &mut Session, conn: &mut Conn) -> std::io::Result<()> {
    while session.stepper_paused() {
        let Some(msg) = crate::rpc::read_message(&mut conn.stream)? else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "connection closed while the stepper was paused",
            ));
        };
        let (method, params, id) = match crate::rpc::parse_request(&msg) {
            Ok(x) => x,
            Err(e) => {
                crate::rpc::write_message(&mut conn.stream, &reply_err(None, &e))?;
                continue;
            }
        };
        if method == Method::Close {
            return Ok(());
        }
        let reply = dispatch(session, conn, method, &params, id.as_ref());
        crate::rpc::write_message(&mut conn.stream, &reply)?;
        drain(session, conn);
        drain_stepper_updates(session, conn)?;
        drain_pending_signals(session, conn)?;
    }
    Ok(())
}

/// A `run`/`run-full-time`/`run-until-time` driven in slices. The engine stops
/// as soon as a click queues a client-owned device action (a window button's
/// `:action`, e.g. `bst`'s `bst-button-pressed`); the server calls it back,
/// services the window commands it issues, and resumes the run. Without this a
/// task's button handler never ran and its display never changed.
///
/// The same loop services the Environment stepper: a pre-event pause is
/// delivered through the client's `next-event` callback, then client requests
/// are pumped until `step-stepper` resumes the run. Post-event updates use the
/// `update-event` callback and yield before the run continues.
fn run_interleaved(
    session: &mut Session,
    conn: &mut Conn,
    name: &str,
    args: &[Json],
    id: Option<&Json>,
) -> Json {
    let secs = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
    let (limit, full, should_run) = session.run_begin(name, secs);
    if !should_run {
        return reply_ok(
            id,
            Json::Arr(vec![
                Json::Float(session.time_secs()),
                Json::Int(session.run_events() as i64),
                Json::Null,
            ]),
        );
    }
    let model = session.current_model().map(str::to_string);
    if let Err(e) = fire_signal(
        session,
        conn,
        "run-start",
        model.as_deref(),
        vec![Json::Float(session.time_secs())],
    ) {
        return reply_err(id, &e.to_string());
    }
    loop {
        let stop = match session.run_slice_reason(limit, full) {
            Ok(stop) => stop,
            Err(e) => return reply_err(id, &e),
        };
        drain(session, conn);
        if let Err(e) = drain_stepper_updates(session, conn) {
            return reply_err(id, &e.to_string());
        }

        if stop == RunStop::StepperPause || session.stepper_paused() {
            if let Some((cmd, a)) = session.take_stepper_pause() {
                if let Err(e) = call_remote_reentrant(session, conn, &cmd, None, a) {
                    return reply_err(id, &e.to_string());
                }
            }
            if let Err(e) = pump_stepper(session, conn) {
                return reply_err(id, &e.to_string());
            }
            if session.take_stepper_abort() {
                break;
            }
            continue;
        }

        if stop == RunStop::StepperUpdate {
            continue;
        }

        if stop == RunStop::DeviceAction {
            let actions = session.take_pending_device_actions();
            for (cmd, a) in actions {
                if let Err(e) = call_remote_reentrant(session, conn, &cmd, None, a) {
                    return reply_err(id, &e.to_string());
                }
            }
            continue;
        }

        break;
    }
    if let Err(e) = fire_signal(
        session,
        conn,
        "run-stop",
        model.as_deref(),
        vec![Json::Float(session.time_secs())],
    ) {
        return reply_err(id, &e.to_string());
    }
    reply_ok(
        id,
        Json::Arr(vec![
            Json::Float(session.time_secs()),
            Json::Int(session.run_events() as i64),
            Json::Null,
        ]),
    )
}
fn dispatch(
    session: &mut Session,
    conn: &mut Conn,
    method: Method,
    params: &[Json],
    id: Option<&Json>,
) -> Json {
    match method {
        Method::Check => {
            let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
            // `check-command` (`framework/dispatcher.lisp:1235`): no entry →
            // one value `(nil)`; an entry → `(t owned-by-this-evaluator doc)`.
            // The Python client treats `result[0] == None` as "free name"
            // (`actr.py:438`), so a missing command must send `[null]`.
            let result = match session.commands().get(name) {
                Some(c) => {
                    let owned = c.connection.is_some() && c.connection == conn.name;
                    Json::Arr(vec![
                        Json::Bool(true),
                        Json::Bool(owned),
                        Json::Str("".into()),
                    ])
                }
                None => Json::Arr(vec![Json::Null]),
            };
            reply_ok(id, result)
        }
        Method::Add => {
            let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
            // `add` params are (name underlying-function documentation
            // single-instance local-name encoded) (`dispatcher.lisp:1700`).
            // The client is called by its *underlying function* (the Tcl
            // proc name), not the local ACT-R macro alias, so prefer
            // params[1]. Fall back to params[4] for older/Python callers that
            // only supplied the alias.
            let local = params
                .get(1)
                .and_then(|p| p.as_str())
                .or_else(|| params.get(4).and_then(|p| p.as_str()))
                .map(str::to_string);
            session.add_remote(name, local, conn.name.clone());
            // `add-command` returns the command name (`actr.py:118` reads
            // `result[1]`).
            reply_ok(
                id,
                Json::Arr(vec![Json::Bool(true), Json::Str(name.to_string())]),
            )
        }
        Method::Remove => {
            let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
            let ok = session.remove(name);
            reply_ok(id, Json::Arr(vec![Json::Bool(ok)]))
        }
        Method::Monitor => {
            let target = params.first().and_then(|p| p.as_str()).unwrap_or("");
            let monitor = params.get(1).and_then(|p| p.as_str()).unwrap_or("");
            let when = match params.get(2).and_then(|p| p.as_str()) {
                Some("before") => MonitorWhen::Before,
                Some("after") => MonitorWhen::After,
                _ => MonitorWhen::Simple,
            };
            session.add_monitor(target, monitor, when);
            reply_ok(id, Json::Arr(vec![Json::Bool(true)]))
        }
        Method::RemoveMonitor => {
            let target = params.first().and_then(|p| p.as_str()).unwrap_or("");
            let monitor = params.get(1).and_then(|p| p.as_str()).unwrap_or("");
            session.remove_monitor(target, monitor);
            reply_ok(id, Json::Arr(vec![Json::Bool(true)]))
        }
        Method::ListCommands => {
            let items: Vec<Json> = session
                .commands()
                .iter()
                .map(|(n, c)| {
                    let kind = match &c.owner {
                        Owner::Builtin => "internal",
                        Owner::None => "none",
                        Owner::Remote { .. } => "remote",
                    };
                    Json::Obj(vec![
                        ("name".into(), Json::Str(n.clone())),
                        ("type".into(), Json::Str(kind.into())),
                    ])
                })
                .collect();
            reply_ok(id, Json::Arr(vec![Json::Arr(items)]))
        }
        Method::ListConnections => {
            let commands: Vec<Json> = session
                .commands()
                .iter()
                .filter(|(_, c)| {
                    c.connection.is_some()
                        && conn.name.is_some()
                        && c.connection.as_deref() == conn.name.as_deref()
                })
                .map(|(n, _)| Json::Str(n.clone()))
                .collect();
            let entry = Json::Arr(vec![
                Json::Str(conn.name.clone().unwrap_or_else(|| conn.peer.clone())),
                Json::Int(0),
                Json::Int(0),
                Json::Int(0),
                Json::Arr(commands),
            ]);
            reply_ok(id, Json::Arr(vec![Json::Arr(vec![entry])]))
        }
        Method::SetName => {
            conn.name = params.first().and_then(|p| p.as_str()).map(str::to_string);
            reply_ok(id, Json::Arr(vec![Json::Bool(true)]))
        }
        Method::Evaluate => {
            let name = match params.first().and_then(|p| p.as_str()) {
                Some(n) => n.to_string(),
                None => return reply_err(id, "evaluate needs a command name"),
            };
            let model = params.get(1).and_then(|p| p.as_str());
            let args = if params.len() > 2 {
                params[2..].to_vec()
            } else {
                Vec::new()
            };
            let result = if matches!(name.as_str(), "run" | "run-full-time" | "run-until-time") {
                return run_interleaved(session, conn, &name, &args, id);
            } else {
                session.evaluate(&name, model, &args, conn)
            };
            // Stream any trace the command produced to the trace monitors, and
            // dispatch the commands the model itself fired (`output-key`, …)
            // to their monitors. ACT-R fires those *inside* the run, so a task
            // reads its handler's effect as soon as `run` returns; a handler
            // can in turn emit trace, so drain until both are quiet.
            drain(session, conn);
            match result {
                Ok(values) => reply_ok(id, Json::Arr(values)),
                Err(e) => reply_err(id, &e),
            }
        }
        Method::Close => reply_ok(id, Json::Null),
    }
}

/// The client connection; also the [`Remote`] implementation (sends an
/// `evaluate` callback and blocks for its reply).
struct Conn {
    stream: TcpStream,
    next_id: i64,
    name: Option<String>,
    peer: String,
}

impl Remote for Conn {
    fn call(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        let id = self.next_id;
        self.next_id += 1;
        let msg = callback(name, model, args, id);
        write_message(&mut self.stream, &msg).map_err(|e| e.to_string())?;
        // Read until the matching reply (the client may interleave nothing
        // else while it is running our callback).
        loop {
            let reply = read_message(&mut self.stream)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "connection closed waiting for a command reply".to_string())?;
            let rid = reply.get("id").and_then(|i| match i {
                Json::Int(n) => Some(*n),
                _ => None,
            });
            if rid == Some(id) {
                return parse_reply(&reply);
            }
            // A stray request from the client that arrived mid-callback: a
            // well-behaved client won't, so drop it.
        }
    }
}

impl std::fmt::Debug for Conn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conn").field("name", &self.name).finish()
    }
}

/// Ensure the stream is flushed (used by tests).
pub fn flush(conn: &mut impl Write) -> std::io::Result<()> {
    conn.flush()
}
