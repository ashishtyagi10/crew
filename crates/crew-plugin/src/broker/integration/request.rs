//! Turning a tool call into an HTTP request — all of it pure, none of it networked.
//!
//! This is where a manifest meets the model's arguments, and it is the half worth testing: a URL
//! assembled wrong is a request to somebody else's server, and a placeholder left unfilled is a
//! literal `{city}` sent as a city name.
use std::collections::BTreeMap;

use super::{Auth, IntTool, Integration};

/// A request, built and ready to send.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Req {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Build the request for `tool` from the model's `args`.
///
/// `Err` is returned TO THE AGENT, so every message says what to do differently: a missing
/// argument names the argument, and a missing credential names the environment variable rather
/// than saying "unauthorized" after a round trip.
pub(crate) fn build(int: &Integration, tool: &IntTool, args: &str) -> Result<Req, String> {
    let vals = arguments(args)?;
    // Every missing argument at once, before the first template asks for one.
    super::args::check(&int.name, tool, &vals)?;
    let mut url = format!(
        "{}{}",
        int.base_url.trim_end_matches('/'),
        fill(&tool.path, &vals)?
    );
    let mut query: Vec<(String, String)> = Vec::new();
    for (k, v) in &tool.query {
        query.push((k.clone(), fill(v, &vals)?));
    }
    let mut headers: Vec<(String, String)> = int
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    match &int.auth {
        Auth::Bearer { env } => {
            headers.push(("Authorization".into(), format!("Bearer {}", secret(env)?)))
        }
        Auth::Header { name, env } => headers.push((name.clone(), secret(env)?)),
        Auth::Query { name, env } => query.push((name.clone(), secret(env)?)),
        Auth::None => {}
    }
    if !query.is_empty() {
        let joined: Vec<String> = query
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
            .collect();
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&joined.join("&"));
    }
    let body = match &tool.body {
        None => None,
        Some(template) => {
            let filled = fill_json(template, &vals)?;
            headers.push(("Content-Type".into(), "application/json".into()));
            Some(filled.to_string())
        }
    };
    Ok(Req {
        method: tool.method.to_uppercase(),
        url,
        headers,
        body,
    })
}

/// The call's arguments as strings. Numbers and booleans are stringified rather than refused:
/// a model that answers `{"lat": 59.9}` has answered correctly, and a URL is text either way.
fn arguments(args: &str) -> Result<BTreeMap<String, String>, String> {
    let args = args.trim();
    if args.is_empty() {
        return Ok(BTreeMap::new());
    }
    let v: serde_json::Value = serde_json::from_str(args)
        .map_err(|e| format!("tool arguments are not valid JSON: {e}"))?;
    let obj = v
        .as_object()
        .ok_or("tool arguments must be a JSON object")?;
    Ok(obj
        .iter()
        .map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            (k.clone(), s)
        })
        .collect())
}

/// Replace every `{name}` in `text` from `vals`.
fn fill(text: &str, vals: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|i| open + i) else {
            // An unmatched brace is a literal one — some APIs really do have them in paths.
            break;
        };
        let name = &rest[open + 1..close];
        let value = vals
            .get(name)
            .ok_or_else(|| format!("missing argument {name:?}"))?;
        out.push_str(&rest[..open]);
        out.push_str(&encode_path(value));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The same substitution inside a JSON body template, on string values only. Structure comes
/// from the manifest; only the leaves are the model's.
fn fill_json(
    v: &serde_json::Value,
    vals: &BTreeMap<String, String>,
) -> Result<serde_json::Value, String> {
    Ok(match v {
        serde_json::Value::String(s) => {
            // A value that is EXACTLY one placeholder keeps the argument's own JSON type, so
            // `{"count": "{n}"}` with `n = 3` sends a number rather than the string "3".
            if let Some(name) = s.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
                let raw = vals
                    .get(name)
                    .ok_or_else(|| format!("missing argument {name:?}"))?;
                return Ok(serde_json::from_str(raw)
                    .unwrap_or_else(|_| serde_json::Value::String(raw.clone())));
            }
            serde_json::Value::String(fill_raw(s, vals)?)
        }
        serde_json::Value::Array(a) => serde_json::Value::Array(
            a.iter()
                .map(|x| fill_json(x, vals))
                .collect::<Result<_, _>>()?,
        ),
        serde_json::Value::Object(o) => serde_json::Value::Object(
            o.iter()
                .map(|(k, x)| Ok((k.clone(), fill_json(x, vals)?)))
                .collect::<Result<_, String>>()?,
        ),
        other => other.clone(),
    })
}

/// [`fill`] without URL encoding — a body is not a URL.
fn fill_raw(text: &str, vals: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|i| open + i) else {
            break;
        };
        let name = &rest[open + 1..close];
        let value = vals
            .get(name)
            .ok_or_else(|| format!("missing argument {name:?}"))?;
        out.push_str(&rest[..open]);
        out.push_str(value);
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The credential named by `env`, or a message that names it. Never the value in a manifest:
/// there is no field that could hold one.
fn secret(env: &str) -> Result<String, String> {
    std::env::var(env)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("{env} is not set \u{2014} this integration needs it to sign in"))
}

/// Percent-encode a query component: everything outside the unreserved set.
pub(crate) fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "%20".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// The same, for a path segment — `/` included, because an argument that contains one would
/// otherwise reach a different endpoint than the manifest describes.
fn encode_path(s: &str) -> String {
    encode(s)
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
