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
    /// Not a modern server (any other error, or silence): use `initialize`.
    Legacy,
    /// A modern server with no version in common. Falling back to
    /// `initialize` is forbidden here, so this ends the connection.
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
        Ok(result) => match common(result.get("supportedVersions")) {
            Some(v) => Probe::Modern(v),
            None => Probe::Refused(mismatch(result.get("supportedVersions"))),
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

/// The version a legacy server answered `initialize` with, if crew speaks
/// it. The spec says a client that does not support the answer SHOULD
/// disconnect; the error names both sides so the log says why. An answer
/// with no version at all is taken as the one crew offered — such a server
/// worked before and refusing it would only break it.
pub fn accept_legacy(answer: Option<&str>) -> Result<String, String> {
    match answer {
        None => Ok(LEGACY[0].to_string()),
        Some(v) if LEGACY.contains(&v) => Ok(v.to_string()),
        Some(v) => Err(format!(
            "server answered MCP {v}; crew speaks {}",
            [MODERN, LEGACY].concat().join(", ")
        )),
    }
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
