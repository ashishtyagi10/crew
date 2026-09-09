//! Sorting what a server sends into the three things it can be.
//!
//! A response carries an `id` and no `method`; a notification carries a
//! `method` and no `id`; a server-to-client REQUEST carries both and expects
//! an answer. The MCP client only needed the first and dropped the rest. A
//! language server sends all three — `publishDiagnostics` is a notification,
//! and rust-analyzer will ask `workspace/configuration` if let — so the
//! reader routes every message rather than waiting for one.
use std::io::BufRead;

use serde_json::{json, Value};

/// A server-initiated message with no reply expected.
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub method: String,
    pub params: Value,
}

/// One message from the server, sorted.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// The reply to one of our requests — the whole envelope, `id` and all.
    Response(Value),
    Notification(Notification),
    /// The server asking us something. Answered by [`reply_to`].
    ServerRequest {
        id: Value,
        method: String,
        params: Value,
    },
}

/// Which of the three `v` is.
pub fn classify(v: Value) -> Incoming {
    let method = v.get("method").and_then(Value::as_str).map(str::to_string);
    let id = v.get("id").filter(|id| !id.is_null()).cloned();
    match (method, id) {
        (Some(method), Some(id)) => Incoming::ServerRequest {
            id,
            method,
            params: v.get("params").cloned().unwrap_or(Value::Null),
        },
        (Some(method), None) => Incoming::Notification(Notification {
            method,
            params: v.get("params").cloned().unwrap_or(Value::Null),
        }),
        (None, _) => Incoming::Response(v),
    }
}

/// The answer to a server-to-client request.
///
/// `workspace/configuration` gets one `null` per item asked for — "no
/// setting, use your default" — because a server that asked and never hears
/// back may wait on it. Everything else is refused with the standard
/// method-not-found code; crew declares no capability that invites more.
pub fn reply_to(method: &str, id: Value, params: &Value) -> Value {
    match method {
        "workspace/configuration" => {
            let n = params
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            json!({"jsonrpc": "2.0", "id": id, "result": vec![Value::Null; n]})
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32601, "message": format!("crew does not handle {method}")},
        }),
    }
}

/// Read frames from `r` until the stream ends or `sink` returns `false`.
/// Undecodable bytes end the loop too: a server that has stopped framing is
/// a server we cannot talk to.
pub fn pump<R: BufRead>(mut r: R, mut sink: impl FnMut(Incoming) -> bool) {
    while let Ok(Some(v)) = super::framing::decode(&mut r) {
        if !sink(classify(v)) {
            break;
        }
    }
}

#[cfg(test)]
#[path = "demux_tests.rs"]
mod tests;
