//! Which MCP protocol revision crew speaks with one server, and what that
//! revision asks of each request.
//!
//! MCP 2026-07-28 dropped the `initialize` handshake: every request carries
//! its own protocol version and client capabilities in `_meta`, and a
//! `server/discover` probe tells a modern server from a legacy one. Servers
//! written against 2025-11-25 and earlier still expect `initialize`, and most
//! servers in the wild are those — so crew is dual-era: probe first, fall back
//! to `initialize` when the probe is not understood. Pure functions here; the
//! wire lives in `handshake.rs` and `client.rs`.
use serde_json::{json, Value};

/// Per-request-metadata revisions crew implements, newest first.
pub const MODERN: &[&str] = &["2026-07-28"];

/// `initialize`-handshake revisions crew can speak, newest first. 2025-03-26
/// asks every client to accept JSON-RPC batches; the reader unpacks them.
pub const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// `UnsupportedProtocolVersionError`: a modern server refusing the version a
/// request named, with `data.supported` listing what it does speak.
pub const UNSUPPORTED: i64 = -32022;

/// What the `server/discover` probe made of the server.
#[derive(Debug, PartialEq, Eq)]
pub enum Probe {
    /// A modern server, and the version both sides speak.
    Modern(String),
    /// Not a modern server (any other error, silence, or an answer with no
    /// version list — bridges that reply `{}` to every method): `initialize`.
    Legacy,
    /// A modern server whose version list shares nothing with crew's.
    /// Falling back to `initialize` is forbidden here, so this ends the
    /// connection.
    Refused(String),
}

/// True when `version` travels in `_meta` rather than an `initialize`.
pub fn is_modern(version: &str) -> bool {
    MODERN.contains(&version)
}

/// The `_meta` every modern request MUST carry: protocol version and client
/// capabilities are required; client identity is a SHOULD crew honours.
/// Capabilities are empty — crew answers no elicitation or sampling.
pub fn meta(version: &str) -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": version,
        "io.modelcontextprotocol/clientInfo":
            {"name": "crew", "version": env!("CARGO_PKG_VERSION")},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

/// `params` with the modern `_meta` merged in (a legacy request goes as-is).
pub fn stamp(mut params: Value, version: &str) -> Value {
    if !is_modern(version) {
        return params;
    }
    if !params.is_object() {
        params = json!({});
    }
    params["_meta"] = meta(version);
    params
}

/// Read the probe's reply: `Ok(result)` or `Err(error object)`.
pub fn classify(reply: Result<&Value, &Value>) -> Probe {
    match reply {
        // Only a real list proves a modern server: hand-written servers and
        // stdio bridges (mcp-remote, supergateway) answer any method with
        // `{}` or `null`, and refusing them would break what worked before.
        Ok(result) => match result.get("supportedVersions") {
            Some(list) if list.is_array() => match common(Some(list)) {
                Some(v) => Probe::Modern(v),
                None => Probe::Refused(mismatch(Some(list))),
            },
            _ => Probe::Legacy,
        },
        Err(err) if err.get("code").and_then(Value::as_i64) == Some(UNSUPPORTED) => {
            let offered = err.get("data").and_then(|d| d.get("supported"));
            match common(offered) {
                Some(v) => Probe::Modern(v),
                None => Probe::Refused(mismatch(offered)),
            }
        }
        // Keyed to no particular code on purpose: legacy servers answer a
        // pre-`initialize` request with whatever error they like.
        Err(_) => Probe::Legacy,
    }
}

/// The newest modern version crew speaks that `offered` also lists.
fn common(offered: Option<&Value>) -> Option<String> {
    let list = offered?.as_array()?;
    MODERN
        .iter()
        .find(|v| list.iter().any(|o| o.as_str() == Some(v)))
        .map(|v| v.to_string())
}

fn mismatch(offered: Option<&Value>) -> String {
    let theirs: Vec<&str> = offered
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let theirs = if theirs.is_empty() {
        "no version".to_string()
    } else {
        theirs.join(", ")
    };
    format!(
        "server speaks MCP {theirs}; crew speaks {}",
        MODERN.join(", ")
    )
}

/// The wire to speak after a legacy server answered `initialize` with
/// `answer`. One of crew's legacy versions is kept; anything else (a draft
/// string, a modern version, nothing at all) is still accepted, as crew always
/// has, but spoken to on the newest legacy wire — never stored verbatim, since
/// a modern version there would stamp `_meta` on a legacy session.
pub fn accept_legacy(answer: Option<&str>) -> String {
    match answer {
        Some(v) if LEGACY.contains(&v) => v.to_string(),
        _ => LEGACY[0].to_string(),
    }
}

/// The one log line for an `initialize` answer crew does not know, or `None`
/// when the answer is one crew speaks (or absent).
pub fn stray(answer: Option<&str>) -> Option<String> {
    let v = answer.filter(|v| !LEGACY.contains(v))?;
    Some(format!(
        "crew: MCP server answered initialize with {v}; speaking {}",
        LEGACY[0]
    ))
}

/// A result's `resultType`: absent (every pre-2026 server) or `"complete"`
/// passes through. `"input_required"` asks for elicitation crew never
/// declared, and an unknown type MUST be treated as invalid — both are errors
/// rather than tool output.
pub fn complete(method: &str, result: Value) -> Result<Value, String> {
    match result.get("resultType").and_then(Value::as_str) {
        None | Some("complete") => Ok(result),
        Some(t) => Err(format!("{method}: unsupported resultType {t:?}")),
    }
}

#[cfg(test)]
#[path = "version_tests.rs"]
mod tests;
