//! The RPC listener: accept clients, read `\x04`-framed JSON requests, and
//! dispatch them through one shared [`Session`].
//!
//! ACT-R's Lisp dispatcher has one global command table and one global model
//! state; connections are clients of that dispatcher.  This server follows the
//! same model.  Connection reader threads only parse frames and send events to
//! a single session actor, which owns the non-`Send` [`Session`] and serializes
//! mutations.
//!
//! A client request may trigger a serverâ†’client `evaluate` callback (a monitor
//! or a client-owned command).  The actor blocks for that reply while still
//! servicing nested requests from the callback, mirroring the Lisp
//! dispatcher's `send-remote-command` (`framework/dispatcher.lisp:2074`).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::time::Duration;

use crate::json::Json;
use crate::rpc::remote_module::ModuleHost;
use crate::rpc::session::{MonitorWhen, Owner, Remote, RunStop, Session};
use crate::rpc::{
    callback, parse_reply, parse_request, reply_err, reply_ok, write_message, Method,
};

/// Serve clients on `addr` (TCP).  Model and task files are loaded from
/// filesystem paths supplied by the client.
pub fn serve(addr: &str, seed: (u64, u64)) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    serve_listener(listener, seed)
}

/// Serve an already-bound listener. Tests use this with port 0 so they can
/// read the assigned address without racing a spawned process.
///
/// All connections share one [`Session`], owned by the actor thread.  The
/// accept loop and per-connection readers only exchange messages with it.
pub fn serve_listener(listener: TcpListener, seed: (u64, u64)) -> std::io::Result<()> {
    let address = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or_default();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || Actor::new(Session::new(seed), rx, address).run());
    let mut next_id = 0u64;
    for stream in listener.incoming() {
        let stream = stream?;
        let id = next_id;
        next_id += 1;
        let peer = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();
        let writer = stream.try_clone()?;
        if tx.send(Event::Connected { id, peer, writer }).is_err() {
            break;
        }
        let tx = tx.clone();
        std::thread::spawn(move || read_connection(id, stream, tx));
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

enum Event {
    Connected {
        id: u64,
        peer: String,
        writer: TcpStream,
    },
    Request {
        id: u64,
        method: Method,
        params: Vec<Json>,
        request_id: Option<Json>,
    },
    BadRequest {
        id: u64,
        message: String,
    },
    Reply {
        id: u64,
        message: Json,
    },
    Disconnected {
        id: u64,
    },
}

/// Read framed messages from one client. Requests go to the actor; callback
/// replies are tagged with the connection id so the actor can route them back
/// to the pending remote command.
fn read_connection(id: u64, mut stream: TcpStream, tx: mpsc::Sender<Event>) {
    let mut buf = Vec::new();
    loop {
        let msg = match read_next_blocking(&mut stream, &mut buf) {
            Ok(Some(msg)) => msg,
            Ok(None) => break,
            Err(e) => {
                eprintln!("rpc: connection {id} ended: {e}");
                break;
            }
        };
        if msg.get("method").is_some() {
            match parse_request(&msg) {
                Ok((method, params, request_id)) => {
                    if tx
                        .send(Event::Request {
                            id,
                            method,
                            params,
                            request_id,
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                Err(e) => {
                    if tx.send(Event::BadRequest { id, message: e }).is_err() {
                        return;
                    }
                }
            }
        } else if msg.get("id").is_some() {
            if tx.send(Event::Reply { id, message: msg }).is_err() {
                return;
            }
        } else if tx
            .send(Event::BadRequest {
                id,
                message: "message has no method".into(),
            })
            .is_err()
        {
            return;
        }
    }
    let _ = tx.send(Event::Disconnected { id });
}

fn read_next_blocking(stream: &mut TcpStream, buf: &mut Vec<u8>) -> std::io::Result<Option<Json>> {
    if let Some(msg) = take_buffered(buf)? {
        return Ok(Some(msg));
    }
    let mut tmp = [0u8; 8192];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return Ok(None),
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(msg) = take_buffered(buf)? {
                    return Ok(Some(msg));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

fn take_buffered(buf: &mut Vec<u8>) -> std::io::Result<Option<Json>> {
    let Some(pos) = buf.iter().position(|b| *b == crate::rpc::EOM) else {
        return Ok(None);
    };
    let frame: Vec<u8> = buf.drain(..=pos).collect();
    let text = String::from_utf8_lossy(&frame[..frame.len() - 1]);
    let v = crate::json::parse(text.trim()).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("bad JSON: {e}"))
    })?;
    Ok(Some(v))
}

/// One live connection as seen by the actor.
/// The transport handles a connection callback needs. Cloned into synchronous
/// module hosts, which run while the actor/session is borrowed.
#[derive(Clone)]
struct ConnIo {
    writer: Rc<RefCell<TcpStream>>,
    next_callback_id: Rc<Cell<i64>>,
    sent: Rc<Cell<u64>>,
}

impl ConnIo {
    fn new(writer: TcpStream) -> ConnIo {
        ConnIo {
            writer: Rc::new(RefCell::new(writer)),
            next_callback_id: Rc::new(Cell::new(1)),
            sent: Rc::new(Cell::new(0)),
        }
    }

    fn next_callback_id(&self) -> i64 {
        let id = self.next_callback_id.get();
        self.next_callback_id.set(id + 1);
        id
    }

    fn send(&self, message: &Json) -> Result<(), String> {
        write_message(&mut *self.writer.borrow_mut(), message).map_err(|e| e.to_string())?;
        self.sent.set(self.sent.get() + 1);
        Ok(())
    }
}

/// Shared event plumbing for the actor and synchronous module hosts.
#[derive(Clone)]
struct CallbackShared {
    rx: Rc<RefCell<Receiver<Event>>>,
    deferred: Rc<RefCell<VecDeque<Event>>>,
}

struct ConnState {
    /// Stable owner key. Display names can change with `set-name`; commands
    /// and monitors stay attached to this key.
    key: String,
    name: Option<String>,
    peer: String,
    io: ConnIo,
    received: u64,
}

impl ConnState {
    fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.peer.clone())
    }
}

/// The single-threaded owner of the shared [`Session`].
struct Actor {
    session: Session,
    rx: Rc<RefCell<Receiver<Event>>>,
    conns: BTreeMap<u64, ConnState>,
    /// Events seen while a remote callback was waiting.  They are handled
    /// after the callback returns instead of being lost.
    deferred: Rc<RefCell<VecDeque<Event>>>,
    /// Prevent nested commands from recursively draining module callbacks.
    module_callback_depth: usize,
    address: String,
}

impl Actor {
    fn new(session: Session, rx: Receiver<Event>, address: String) -> Actor {
        Actor {
            session,
            rx: Rc::new(RefCell::new(rx)),
            conns: BTreeMap::new(),
            deferred: Rc::new(RefCell::new(VecDeque::new())),
            module_callback_depth: 0,
            address,
        }
    }

    fn run(mut self) {
        loop {
            let next = self.rx.borrow_mut().recv();
            let Ok(event) = next else {
                break;
            };
            self.handle_event(event);
            self.process_deferred();
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Connected { id, peer, writer } => {
                self.conns.insert(
                    id,
                    ConnState {
                        key: format!("connection-{id}"),
                        name: None,
                        peer,
                        io: ConnIo::new(writer),
                        received: 0,
                    },
                );
            }
            Event::Request {
                id,
                method,
                params,
                request_id,
            } => self.handle_request(id, method, params, request_id),
            Event::BadRequest { id, message } => {
                let reply = reply_err(None, &message);
                self.write_reply(id, reply);
            }
            Event::Reply { .. } => {
                // A callback reply with no outstanding callback. Ignore it.
            }
            Event::Disconnected { id } => self.disconnect(id),
        }
    }

    fn process_deferred(&mut self) {
        loop {
            let event = self.deferred.borrow_mut().pop_front();
            let Some(event) = event else {
                break;
            };
            self.handle_event(event);
        }
    }

    fn handle_request(
        &mut self,
        conn_id: u64,
        method: Method,
        params: Vec<Json>,
        request_id: Option<Json>,
    ) {
        let Some(conn) = self.conns.get_mut(&conn_id) else {
            return;
        };
        conn.received += 1;
        if std::env::var("RPC_DEBUG").is_ok() {
            eprintln!(
                "rpc<< {:?} {:?} id={:?}",
                method,
                params.first().and_then(|p| p.as_str()).unwrap_or(""),
                request_id
            );
        }
        if method == Method::Close {
            self.disconnect(conn_id);
            return;
        }

        let reply = self.dispatch(conn_id, method, &params, request_id.as_ref());
        if self.conns.contains_key(&conn_id) {
            if let Err(e) = self.drain_pending_signals(conn_id) {
                eprintln!("rpc: pending signals for connection {conn_id}: {e}");
            }
            // A `reset` defers a client-owned `:testing-function` to here
            // (`model-tester.cl:485`); run it with the session borrow released.
            if let Some(name) = self.session.take_pending_testing_fn() {
                let owner = self.owner_key_for(&name);
                let _ = self.call_remote(owner.as_deref(), conn_id, &name, None, Vec::new());
            }
            self.drain_module_callbacks(conn_id);
            self.write_reply(conn_id, reply);
        }
    }

    fn dispatch(
        &mut self,
        conn_id: u64,
        method: Method,
        params: &[Json],
        id: Option<&Json>,
    ) -> Json {
        match method {
            Method::Check => {
                let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
                // `check-command` (`framework/dispatcher.lisp:1235`): no entry â†’
                // one value `(nil)`; an entry â†’ `(t owned-by-this-evaluator doc)`.
                // The Python client treats `result[0] == None` as "free name"
                // (`actr.py:438`), so a missing command must send `[null]`.
                let result = match self.session.commands().get(name) {
                    Some(c) => {
                        let owned = self
                            .conns
                            .get(&conn_id)
                            .is_some_and(|conn| c.connection.as_deref() == Some(&conn.key));
                        Json::Arr(vec![
                            Json::Bool(true),
                            Json::Bool(owned),
                            Json::Str(c.doc.clone()),
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
                let key = self
                    .conns
                    .get(&conn_id)
                    .map(|conn| conn.key.clone())
                    .unwrap_or_default();
                match self.session.add_remote_checked(name, local, Some(key)) {
                    Ok(()) => {
                        if let Some(doc) = params.get(2).and_then(|p| p.as_str()) {
                            self.session.set_command_doc(name, doc);
                        }
                        // `add-command` returns the command name (`actr.py:118`
                        // reads `result[1]`).
                        reply_ok(
                            id,
                            Json::Arr(vec![Json::Bool(true), Json::Str(name.to_string())]),
                        )
                    }
                    Err(error) => {
                        reply_ok(id, Json::Arr(vec![Json::Bool(false), Json::Str(error)]))
                    }
                }
            }
            Method::Remove => {
                let name = params.first().and_then(|p| p.as_str()).unwrap_or("");
                match self.session.remove_checked(name) {
                    Ok(()) => reply_ok(id, Json::Arr(vec![Json::Bool(true)])),
                    Err(error) => {
                        reply_ok(id, Json::Arr(vec![Json::Bool(false), Json::Str(error)]))
                    }
                }
            }
            Method::Monitor => {
                let target = params.first().and_then(|p| p.as_str()).unwrap_or("");
                let monitor = params.get(1).and_then(|p| p.as_str()).unwrap_or("");
                let when = match params.get(2).and_then(|p| p.as_str()) {
                    Some("before") => MonitorWhen::Before,
                    Some("after") => MonitorWhen::After,
                    _ => MonitorWhen::Simple,
                };
                self.session.add_monitor(target, monitor, when);
                reply_ok(id, Json::Arr(vec![Json::Bool(true)]))
            }
            Method::RemoveMonitor => {
                let target = params.first().and_then(|p| p.as_str()).unwrap_or("");
                let monitor = params.get(1).and_then(|p| p.as_str()).unwrap_or("");
                self.session.remove_monitor(target, monitor);
                reply_ok(id, Json::Arr(vec![Json::Bool(true)]))
            }
            Method::ListCommands => {
                let items: Vec<Json> = self
                    .session
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
                let mut rows = Vec::new();
                for conn in self.conns.values() {
                    let commands: Vec<Json> = self
                        .session
                        .commands()
                        .iter()
                        .filter(|(_, c)| c.connection.as_deref() == Some(conn.key.as_str()))
                        .map(|(n, _)| Json::Str(n.clone()))
                        .collect();
                    // Lisp row shape (`list-of-act-r-connections`):
                    // name, address, received/pending count, sent count,
                    // commands.
                    rows.push(Json::Arr(vec![
                        Json::Str(conn.display_name()),
                        Json::Str(conn.peer.clone()),
                        Json::Int(conn.received as i64),
                        Json::Int(conn.io.sent.get() as i64),
                        Json::Arr(commands),
                    ]));
                }
                let internal: Vec<Json> = self
                    .session
                    .commands()
                    .iter()
                    .filter(|(_, c)| matches!(c.owner, Owner::Builtin))
                    .map(|(n, _)| Json::Str(n.clone()))
                    .collect();
                rows.push(Json::Arr(vec![
                    Json::Str("internal ACT-R commands".into()),
                    Json::Str(self.address.clone()),
                    Json::Int(-1),
                    Json::Int(-1),
                    Json::Arr(internal),
                ]));
                reply_ok(id, Json::Arr(vec![Json::Arr(rows)]))
            }
            Method::SetName => {
                if let Some(conn) = self.conns.get_mut(&conn_id) {
                    conn.name = params.first().and_then(|p| p.as_str()).map(str::to_string);
                }
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
                if matches!(name.as_str(), "run" | "run-full-time" | "run-until-time") {
                    return self.run_interleaved(conn_id, &name, &args, id);
                }
                let result = self.evaluate(conn_id, &name, model, &args);
                // Stream any trace the command produced to the trace monitors,
                // and dispatch the commands the model itself fired
                // (`output-key`, â€¦) to their monitors. ACT-R fires those
                // *inside* the run, so a task reads its handler's effect as soon
                // as `run` returns; a handler can in turn emit trace, so drain
                // until both are quiet.
                self.drain(conn_id);
                match result {
                    Ok(values) => reply_ok(id, Json::Arr(values)),
                    Err(e) => reply_err(id, &e),
                }
            }
            Method::Close => reply_ok(id, Json::Null),
        }
    }

    fn evaluate(
        &mut self,
        origin: u64,
        name: &str,
        model: Option<&str>,
        args: &[Json],
    ) -> Result<Vec<Json>, String> {
        self.with_remote(origin, |session, remote| {
            session.evaluate(name, model, args, remote)
        })
    }

    fn with_remote<T>(
        &mut self,
        origin: u64,
        f: impl FnOnce(&mut Session, &mut dyn Remote) -> T,
    ) -> T {
        let Actor {
            session,
            conns,
            rx,
            deferred,
            ..
        } = self;
        let mut remote = ActorRemote {
            conns,
            shared: CallbackShared {
                rx: rx.clone(),
                deferred: deferred.clone(),
            },
            origin,
        };
        f(session, &mut remote)
    }

    /// Invoke a client-owned command and service requests made from inside it.
    /// The session borrow is released here, so a nested request can dispatch
    /// (`paired-learning`'s testing function calls `set-parameter-value`).
    fn call_remote(
        &mut self,
        owner: Option<&str>,
        origin: u64,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        // The wire protocol expects the client's underlying proc name, not the
        // dispatcher command name the Tcl Environment registered.
        let call = self.session.remote_local_name(name).to_string();
        let target = self.target_conn(owner, origin)?;
        let callback_id = {
            let conn = self
                .conns
                .get_mut(&target)
                .ok_or_else(|| "callback connection is closed".to_string())?;
            let id = conn.io.next_callback_id();
            let msg = callback(&call, model, args, id);
            conn.io.send(&msg)?;
            id
        };
        loop {
            self.process_deferred();
            if !self.conns.contains_key(&target) {
                return Err("connection closed waiting for a command reply".into());
            }
            let next = self.rx.borrow_mut().recv();
            match next {
                Ok(Event::Reply { id, message })
                    if id == target && message_id(&message) == Some(callback_id) =>
                {
                    return parse_reply(&message);
                }
                Ok(Event::Reply { .. }) => {}
                Ok(event) => self.handle_event(event),
                Err(_) => return Err("server dispatcher stopped".into()),
            }
        }
    }

    fn target_conn(&self, owner: Option<&str>, origin: u64) -> Result<u64, String> {
        if let Some(owner) = owner {
            return self
                .conns
                .iter()
                .find(|(_, conn)| conn.key == owner)
                .map(|(id, _)| *id)
                .ok_or_else(|| format!("connection {owner:?} is closed"));
        }
        if self.conns.contains_key(&origin) {
            Ok(origin)
        } else {
            Err("connection is closed".into())
        }
    }

    fn owner_key_for(&self, command: &str) -> Option<String> {
        self.session
            .commands()
            .get(command)
            .and_then(|c| c.connection.clone())
    }

    /// Run deferred module side-effect callbacks with the session borrow
    /// released. Nested commands issued by a callback are serviced by
    /// `call_remote`; callbacks they queue are picked up by this loop rather
    /// than recursively draining.
    fn drain_module_callbacks(&mut self, origin: u64) {
        if self.module_callback_depth > 0 {
            return;
        }
        self.module_callback_depth += 1;
        for _ in 0..32 {
            let callbacks = self.session.take_pending_module_callbacks_with_models();
            if callbacks.is_empty() {
                break;
            }
            for (owner, model, command, args) in callbacks {
                let mut params = vec![Json::Str(model.clone())];
                params.extend(args);
                let owner = owner.or_else(|| self.owner_key_for(&command));
                if let Err(error) =
                    self.call_remote(owner.as_deref(), origin, &command, Some(&model), params)
                {
                    eprintln!("rpc: module callback {command} for {model} failed: {error}");
                }
            }
        }
        self.module_callback_depth -= 1;
    }

    fn drain(&mut self, origin: u64) {
        for _ in 0..32 {
            let lines = self.session.take_trace_events();
            for (line, kind) in lines {
                // Trace monitors are client-owned. Record the callbacks while
                // `Session` is borrowed, then run them reentrantly so a monitor
                // on another connection can still issue nested requests.
                let mut remote = TraceRemote::default();
                self.session.deliver_trace(&line, kind, &mut remote);
                for (owner, name, model, args) in remote.calls {
                    let _ =
                        self.call_remote(owner.as_deref(), origin, &name, model.as_deref(), args);
                }
            }
            let fired = self.session.take_fired_commands();
            let actions = self.session.take_pending_device_actions_with_models();
            if fired.is_empty() && actions.is_empty() {
                break;
            }
            for (cmd, model_name, key) in fired {
                let a = vec![Json::Str(model_name.clone()), Json::Str(key)];
                for (monitor, remote) in self.session.simple_monitors(&cmd) {
                    if remote {
                        let owner = self.owner_key_for(&monitor);
                        let _ = self.call_remote(
                            owner.as_deref(),
                            origin,
                            &monitor,
                            Some(&model_name),
                            a.clone(),
                        );
                    } else {
                        let _ = self.evaluate(origin, &monitor, Some(&model_name), &a);
                    }
                }
            }
            for (model_name, cmd, args) in actions {
                if self.session.command_is_remote(&cmd) {
                    let owner = self.owner_key_for(&cmd);
                    let _ = self.call_remote(
                        owner.as_deref(),
                        origin,
                        &cmd,
                        model_name.as_deref(),
                        args,
                    );
                } else {
                    let _ = self.evaluate(origin, &cmd, model_name.as_deref(), &args);
                }
            }
        }
    }

    fn fire_signal(
        &mut self,
        origin: u64,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<(), String> {
        for (monitor, remote) in self.session.simple_monitors(name) {
            if remote {
                let owner = self.owner_key_for(&monitor);
                self.call_remote(owner.as_deref(), origin, &monitor, model, args.clone())?;
            } else {
                let _ = self.evaluate(origin, &monitor, model, &args);
            }
        }
        self.drain(origin);
        Ok(())
    }

    fn drain_pending_signals(&mut self, origin: u64) -> Result<(), String> {
        for (name, model, args) in self.session.take_pending_signals() {
            self.fire_signal(origin, &name, model.as_deref(), args)?;
        }
        Ok(())
    }

    fn drain_stepper_updates(&mut self, origin: u64) -> Result<(), String> {
        while let Some((cmd, model, args)) = self.session.take_stepper_update() {
            let owner = self.owner_key_for(&cmd);
            self.call_remote(owner.as_deref(), origin, &cmd, model.as_deref(), args)?;
            self.drain(origin);
        }
        Ok(())
    }

    /// Service ordinary requests between run slices without answering the
    /// pending `run` request.  Any connection may send Stop, inspect state, or
    /// step the shared session.
    fn pump_requests(&mut self, origin: u64) -> Result<(), String> {
        self.process_deferred();
        loop {
            let next = self.rx.borrow_mut().try_recv();
            match next {
                Ok(Event::Request {
                    id,
                    method,
                    params,
                    request_id,
                }) => self.handle_request(id, method, params, request_id),
                Ok(Event::BadRequest { id, message }) => {
                    let reply = reply_err(None, &message);
                    self.write_reply(id, reply);
                }
                Ok(Event::Connected { id, peer, writer }) => {
                    self.handle_event(Event::Connected { id, peer, writer })
                }
                Ok(Event::Disconnected { id }) => self.disconnect(id),
                Ok(Event::Reply { .. }) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Err("server dispatcher stopped".into()),
            }
        }
        if self.conns.contains_key(&origin) {
            Ok(())
        } else {
            Err("connection closed while a run was in progress".into())
        }
    }

    /// While a stepper is paused, service Step / Stop / Update / Set-* from
    /// any connection without answering the pending `run` request.
    fn pump_stepper(&mut self, origin: u64) -> Result<(), String> {
        while self.session.stepper_paused() {
            self.process_deferred();
            if !self.conns.contains_key(&origin) {
                return Err("connection closed while the stepper was paused".into());
            }
            let next = self.rx.borrow_mut().recv();
            match next {
                Ok(event) => self.handle_event(event),
                Err(_) => return Err("server dispatcher stopped".into()),
            }
        }
        Ok(())
    }

    /// A `run`/`run-full-time`/`run-until-time` driven in slices. The engine
    /// stops as soon as a click queues a client-owned device action (a window
    /// button's `:action`, e.g. `bst`'s `bst-button-pressed`); the server calls
    /// it back, services the window commands it issues, and resumes the run.
    ///
    /// The same loop services the Environment stepper from any connection.
    fn run_interleaved(
        &mut self,
        origin: u64,
        name: &str,
        args: &[Json],
        id: Option<&Json>,
    ) -> Json {
        let secs = args.first().and_then(|a| a.as_f64()).unwrap_or(0.0);
        let start_ms = (self.session.time_secs() * 1000.0).round() as i64;
        let (limit, full, should_run) = self.session.run_begin(name, secs);
        self.drain_module_callbacks(origin);
        if !should_run {
            return reply_ok(
                id,
                Json::Arr(vec![Json::Float(0.0), Json::Int(0), Json::Null]),
            );
        }
        let model = self.session.current_model().map(str::to_string);
        if let Err(e) = self.fire_signal(
            origin,
            "run-start",
            model.as_deref(),
            vec![Json::Float(self.session.time_secs())],
        ) {
            return reply_err(id, &e);
        }
        loop {
            let stop = match self.session.run_slice_reason_capped(limit, full, 500) {
                Ok(stop) => stop,
                Err(e) => return reply_err(id, &e),
            };
            self.drain(origin);
            self.drain_module_callbacks(origin);
            if let Err(e) = self.drain_stepper_updates(origin) {
                return reply_err(id, &e);
            }
            // Service Stop / event-queue / inspector requests while the run is
            // in flight. Without this the connection is blocked on the `run`
            // reply.
            if let Err(e) = self.pump_requests(origin) {
                return reply_err(id, &e);
            }

            if stop == RunStop::StepperPause || self.session.stepper_paused() {
                if let Some((cmd, model, a)) = self.session.take_stepper_pause() {
                    let owner = self.owner_key_for(&cmd);
                    if let Err(e) =
                        self.call_remote(owner.as_deref(), origin, &cmd, model.as_deref(), a)
                    {
                        return reply_err(id, &e);
                    }
                }
                if let Err(e) = self.pump_stepper(origin) {
                    return reply_err(id, &e);
                }
                if self.session.take_stepper_abort() {
                    break;
                }
                continue;
            }

            if stop == RunStop::ModuleCallback
                || stop == RunStop::StepperUpdate
                || stop == RunStop::Continue
            {
                continue;
            }

            if stop == RunStop::DeviceAction {
                let actions = self.session.take_pending_device_actions_with_models();
                for (model, cmd, a) in actions {
                    let owner = self.owner_key_for(&cmd);
                    if let Err(e) =
                        self.call_remote(owner.as_deref(), origin, &cmd, model.as_deref(), a)
                    {
                        return reply_err(id, &e);
                    }
                }
                continue;
            }

            break;
        }
        self.drain_module_callbacks(origin);
        if let Err(e) = self.fire_signal(
            origin,
            "run-stop",
            model.as_deref(),
            vec![Json::Float(self.session.time_secs())],
        ) {
            return reply_err(id, &e);
        }
        let elapsed_ms = (self.session.time_secs() * 1000.0).round() as i64 - start_ms;
        reply_ok(
            id,
            Json::Arr(vec![
                Json::Float(elapsed_ms as f64 / 1000.0),
                Json::Int(self.session.run_events() as i64),
                if self.session.run_break_hit() {
                    Json::Bool(true)
                } else {
                    Json::Null
                },
            ]),
        )
    }

    fn write_reply(&mut self, conn_id: u64, reply: Json) {
        let failed = self
            .conns
            .get(&conn_id)
            .map(|conn| conn.io.send(&reply).is_err())
            .unwrap_or(false);
        if failed {
            self.disconnect(conn_id);
        }
    }

    fn disconnect(&mut self, conn_id: u64) {
        if let Some(conn) = self.conns.remove(&conn_id) {
            self.session.remove_connection(&conn.key);
            let _ = conn.io.writer.borrow().shutdown(Shutdown::Both);
        }
    }
}

/// Records client callbacks made while delivering trace.  The actor runs them
/// after releasing the session borrow, so a monitor on another connection can
/// still dispatch nested requests.
#[derive(Default)]
struct TraceRemote {
    calls: Vec<(Option<String>, String, Option<String>, Vec<Json>)>,
}

impl Remote for TraceRemote {
    fn call(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.call_from(None, name, model, args)
    }

    fn call_from(
        &mut self,
        owner: Option<&str>,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.calls.push((
            owner.map(str::to_string),
            name.to_string(),
            model.map(str::to_string),
            args,
        ));
        Ok(vec![Json::Bool(true)])
    }
}

/// The [`Remote`] used while the actor holds `&mut Session`.  It routes a
/// client-owned command to the connection that registered it.
struct ActorRemote<'a> {
    conns: &'a mut BTreeMap<u64, ConnState>,
    shared: CallbackShared,
    origin: u64,
}

impl Remote for ActorRemote<'_> {
    fn call(
        &mut self,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        self.call_from(None, name, model, args)
    }

    fn call_from(
        &mut self,
        owner: Option<&str>,
        name: &str,
        model: Option<&str>,
        args: Vec<Json>,
    ) -> Result<Vec<Json>, String> {
        let target = if let Some(owner) = owner {
            self.conns
                .iter()
                .find(|(_, conn)| conn.key == owner)
                .map(|(id, _)| *id)
                .ok_or_else(|| format!("connection {owner:?} is closed"))?
        } else if self.conns.contains_key(&self.origin) {
            self.origin
        } else {
            return Err("connection is closed".into());
        };
        let callback_id = {
            let conn = self
                .conns
                .get_mut(&target)
                .ok_or_else(|| "callback connection is closed".to_string())?;
            let id = conn.io.next_callback_id();
            let msg = callback(name, model, args, id);
            conn.io.send(&msg)?;
            id
        };
        loop {
            let next = self.shared.rx.borrow_mut().recv();
            match next {
                Ok(Event::Reply { id, message })
                    if id == target && message_id(&message) == Some(callback_id) =>
                {
                    return parse_reply(&message);
                }
                Ok(Event::Disconnected { id }) if id == target => {
                    self.shared
                        .deferred
                        .borrow_mut()
                        .push_back(Event::Disconnected { id });
                    return Err("connection closed waiting for a command reply".into());
                }
                Ok(event) => self.shared.deferred.borrow_mut().push_back(event),
                Err(_) => return Err("server dispatcher stopped".into()),
            }
        }
    }

    fn module_host(&self) -> Option<Rc<dyn ModuleHost>> {
        let conn = self.conns.get(&self.origin)?;
        Some(Rc::new(RpcModuleHost {
            conn_id: self.origin,
            io: conn.io.clone(),
            shared: self.shared.clone(),
        }))
    }

    fn connection_key(&self) -> Option<String> {
        self.conns.get(&self.origin).map(|conn| conn.key.clone())
    }
}

const MODULE_CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);

/// The synchronous host used by remote module data callbacks.
///
/// It runs while `&mut Session` is borrowed, so it talks to the connection
/// directly instead of going through [`Actor::call_remote`]. Nested ACT-R
/// commands are rejected: there is no safe re-entrant session borrow here.
struct RpcModuleHost {
    conn_id: u64,
    io: ConnIo,
    shared: CallbackShared,
}

impl ModuleHost for RpcModuleHost {
    fn invoke(&self, command: &str, model: &str, args: Vec<Json>) -> Result<Vec<Json>, String> {
        let callback_id = self.io.next_callback_id();
        let mut params = vec![Json::Str(model.to_string())];
        params.extend(args);
        self.io
            .send(&callback(command, Some(model), params, callback_id))?;
        loop {
            let next = self
                .shared
                .rx
                .borrow()
                .recv_timeout(MODULE_CALLBACK_TIMEOUT);
            match next {
                Ok(Event::Reply { id, message })
                    if id == self.conn_id && message_id(&message) == Some(callback_id) =>
                {
                    return parse_reply(&message);
                }
                Ok(Event::Request { id, request_id, .. }) if id == self.conn_id => {
                    let _ = self.io.send(&reply_err(
                        request_id.as_ref(),
                        "Nested ACT-R commands are not allowed from synchronous module callbacks.",
                    ));
                }
                Ok(Event::Disconnected { id }) if id == self.conn_id => {
                    self.shared
                        .deferred
                        .borrow_mut()
                        .push_back(Event::Disconnected { id });
                    return Err("connection closed during a module callback".into());
                }
                Ok(event) => self.shared.deferred.borrow_mut().push_back(event),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(format!(
                        "Timed out waiting for module callback {command:?}."
                    ))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("server dispatcher stopped".into())
                }
            }
        }
    }
}

fn message_id(msg: &Json) -> Option<i64> {
    msg.get("id").and_then(|i| match i {
        Json::Int(n) => Some(*n),
        _ => None,
    })
}

impl std::fmt::Debug for ConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnState")
            .field("name", &self.name)
            .finish()
    }
}

/// Ensure the stream is flushed (used by tests).
pub fn flush(conn: &mut impl Write) -> std::io::Result<()> {
    conn.flush()
}
