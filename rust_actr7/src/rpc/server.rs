//! The RPC listener: accept a client, read `\x04`-framed JSON requests,
//! dispatch them through the [`Session`], and stream trace output to monitors.
//!
//! Single-connection, synchronous: a client request may trigger a server→
//! client `evaluate` callback (a monitor or a client-owned command), and the
//! server blocks reading that reply before continuing — the same shape as the
//! Lisp dispatcher's `send-remote-command` (`framework/dispatcher.lisp:2074`).

use std::io::Write;
use std::net::{TcpListener, TcpStream};

use crate::json::Json;
use crate::rpc::session::{MonitorWhen, Owner, Remote, Session};
use crate::rpc::{
    callback, parse_reply, parse_request, read_message, reply_err, reply_ok, write_message, Method,
};

/// Serve one client at a time on `addr` (TCP). `root` is the ACT-R checkout.
pub fn serve(addr: &str, root: std::path::PathBuf, seed: (u64, u64)) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        let stream = stream?;
        let mut session = Session::new(root.clone(), seed);
        if let Err(e) = handle(&mut session, stream) {
            eprintln!("rpc: connection ended: {e}");
        }
    }
    Ok(())
}

/// Write the ACT-R port/host files the Python client looks for
/// (`tutorial/python/actr.py:190`).
pub fn write_port_file(port: u16) -> std::io::Result<()> {
    // `actr.py` does `int(f.readline())` for the port (newline-tolerant) but
    // uses the host line verbatim, so neither may carry a trailing newline.
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::fs::write(format!("{home}/act-r-port-num.txt"), format!("{port}"))?;
    std::fs::write(format!("{home}/act-r-address.txt"), "127.0.0.1")?;
    Ok(())
}

fn handle(session: &mut Session, stream: TcpStream) -> std::io::Result<()> {
    let mut conn = Conn {
        stream,
        next_id: 1,
        name: None,
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
        write_message(&mut conn.stream, &reply)?;
    }
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
            let local = params.get(4).and_then(|p| p.as_str()).map(str::to_string);
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
        Method::ListConnections => reply_ok(id, Json::Arr(vec![Json::Arr(vec![])])),
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
            let result = session.evaluate(&name, model, &args, conn);
            // Stream any trace the command produced to the trace monitors.
            let lines = session.take_trace();
            for line in lines {
                session.deliver_trace(&line, conn);
            }
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
