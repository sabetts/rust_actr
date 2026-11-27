//! ACT-R remote-command dispatcher (the RPC bridge server).
//!
//! Speaks the wire protocol the Python/other-language clients use
//! (`tutorial/python/actr.py` client, `framework/dispatcher.lisp` server):
//! TCP, JSON messages terminated by `\x04`, `{"method","id","params"}` in and
//! `{"id","result","error"}` out. All real work is `evaluate <command-name>
//! <args>` — a name→handler lookup — and a client-registered command fires via
//! a server→client `{"method":"evaluate","params":[name,model,…]}` callback.
//!
//! This module is deliberately transport-first and dependency-free; the
//! command handlers live in [`crate::rpc::session`].

use std::io::{BufRead, Read, Write};

use crate::json::Json;

pub mod builtin_docs;
pub mod remote_module;
pub mod server;
pub mod session;

/// A framed RPC message: the JSON body and the `\x04` terminator.
pub const EOM: u8 = 4;

/// Read one `\x04`-terminated message. Returns `Ok(None)` on a clean EOF.
pub fn read_message<R: Read>(r: &mut R) -> std::io::Result<Option<Json>> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match r.read(&mut byte)? {
            0 => {
                return if buf.is_empty() {
                    Ok(None)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "connection closed mid-message",
                    ))
                };
            }
            _ => {
                if byte[0] == EOM {
                    break;
                }
                buf.push(byte[0]);
            }
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let v = crate::json::parse(text.trim()).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, format!("bad JSON: {e}"))
    })?;
    Ok(Some(v))
}

/// Write one `\x04`-terminated JSON message.
pub fn write_message<W: Write>(w: &mut W, msg: &Json) -> std::io::Result<()> {
    // One write so the frame is less likely to be split across segments.
    let mut buf = msg.write().into_bytes();
    buf.push(EOM);
    w.write_all(&buf)?;
    w.flush()
}

/// A success reply: `{"result": …, "error": null, "id": N}`.
pub fn reply_ok(id: Option<&Json>, result: Json) -> Json {
    Json::Obj(vec![
        ("result".into(), result),
        ("error".into(), Json::Null),
        ("id".into(), id.cloned().unwrap_or(Json::Null)),
    ])
}

/// A failure reply: `{"result": null, "error": {"message": …}, "id": N}`.
pub fn reply_err(id: Option<&Json>, message: &str) -> Json {
    Json::Obj(vec![
        ("result".into(), Json::Null),
        (
            "error".into(),
            Json::Obj(vec![("message".into(), Json::Str(message.to_string()))]),
        ),
        ("id".into(), id.cloned().unwrap_or(Json::Null)),
    ])
}

/// A server→client command callback: `{"method":"evaluate","params":[…],"id":N}`.
pub fn callback(name: &str, model: Option<&str>, params: Vec<Json>, id: i64) -> Json {
    let mut args = vec![Json::Str(name.to_string())];
    args.push(
        model
            .map(|m| Json::Str(m.to_string()))
            .unwrap_or(Json::Null),
    );
    args.extend(params);
    Json::Obj(vec![
        ("method".into(), Json::Str("evaluate".into())),
        ("params".into(), Json::Arr(args)),
        ("id".into(), Json::Int(id)),
    ])
}

/// The dispatcher's `process-external-input` message set
/// (`framework/dispatcher.lisp:1663`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Add,
    Remove,
    Evaluate,
    Monitor,
    RemoveMonitor,
    ListCommands,
    ListConnections,
    Check,
    SetName,
    Close,
}

impl Method {
    pub fn parse(s: &str) -> Option<Method> {
        Some(match s {
            "add" => Method::Add,
            "remove" => Method::Remove,
            "evaluate" => Method::Evaluate,
            "monitor" => Method::Monitor,
            "remove-monitor" => Method::RemoveMonitor,
            "list-commands" => Method::ListCommands,
            "list-connections" => Method::ListConnections,
            "check" => Method::Check,
            "set-name" => Method::SetName,
            "close" => Method::Close,
            _ => return None,
        })
    }
}

/// Split a client request into `(method, params, id)`.
pub fn parse_request(msg: &Json) -> Result<(Method, Vec<Json>, Option<Json>), String> {
    let method = msg
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or("message has no method")?;
    let method = Method::parse(method).ok_or_else(|| format!("invalid method {method:?}"))?;
    let params = match msg.get("params") {
        Some(Json::Arr(a)) => a.clone(),
        Some(Json::Null) | None => Vec::new(),
        _ => return Err("params must be an array".into()),
    };
    let id = msg.get("id").cloned();
    Ok((method, params, id))
}

/// Read the client's reply to a server callback (`{"id","result","error"}`).
pub fn parse_reply(msg: &Json) -> Result<Vec<Json>, String> {
    let err = msg.get("error");
    if let Some(e) = err {
        if !matches!(e, Json::Null) {
            let m = e
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("remote command failed");
            return Err(m.to_string());
        }
    }
    match msg.get("result") {
        Some(Json::Arr(a)) => Ok(a.clone()),
        Some(Json::Null) | None => Ok(Vec::new()),
        Some(other) => Ok(vec![other.clone()]),
    }
}

/// Convenience for line-oriented bring-up logs.
pub fn read_all_messages<R: BufRead>(r: &mut R) -> std::io::Result<Vec<Json>> {
    let mut out = Vec::new();
    while let Some(m) = read_message(r)? {
        out.push(m);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_eom_terminated() {
        let request = r#"{"method":"evaluate","id":3,"params":["run",null,10]}"#;
        let mut bytes = request.as_bytes().to_vec();
        bytes.push(EOM);
        let mut r = std::io::Cursor::new(bytes.clone());
        let msg = read_message(&mut r).unwrap().unwrap();
        let (m, params, id) = parse_request(&msg).unwrap();
        assert_eq!(m, Method::Evaluate);
        assert_eq!(id, Some(Json::Int(3)));
        assert_eq!(params[0].as_str(), Some("run"));
        assert_eq!(params[2].as_f64(), Some(10.0));

        let mut out = Vec::new();
        write_message(&mut out, &msg).unwrap();
        assert_eq!(out, bytes);
    }

    #[test]
    fn success_and_error_envelopes() {
        let ok = reply_ok(Some(&Json::Int(7)), Json::Arr(vec![Json::Bool(true)]));
        assert_eq!(ok.write(), r#"{"result":[true],"error":null,"id":7}"#);
        let err = reply_err(Some(&Json::Int(7)), "boom");
        assert_eq!(
            err.write(),
            r#"{"result":null,"error":{"message":"boom"},"id":7}"#
        );
    }

    #[test]
    fn callback_shape_matches_the_client() {
        let cb = callback(
            "Python-model-tester-output",
            Some("COUNT"),
            vec![Json::Str("hi".into())],
            42,
        );
        assert_eq!(
            cb.write(),
            r#"{"method":"evaluate","params":["Python-model-tester-output","COUNT","hi"],"id":42}"#
        );
    }

    #[test]
    fn reply_parsing() {
        let ok = crate::json::parse(r#"{"id":1,"result":["a","b"],"error":null}"#).unwrap();
        assert_eq!(parse_reply(&ok).unwrap().len(), 2);
        let err = crate::json::parse(r#"{"id":1,"result":null,"error":{"message":"no"}}"#).unwrap();
        assert_eq!(parse_reply(&err).unwrap_err(), "no");
    }
}
