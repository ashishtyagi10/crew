//! Turning an agent CLI's raw stdout into a clean reply string. `claude -p
//! --output-format text` and `codex exec` already print just the reply on
//! stdout (codex's session banner goes to stderr, which the runner discards),
//! so they use [`super::Normalize::Raw`]. opencode emits a stream of JSON event
//! lines; [`opencode_json`] pulls the assistant text out and surfaces errors.
use serde_json::Value;

use super::opencodesteps::{final_step_text, has_steps, silent_summary};

/// Extract the assistant's reply from opencode's `--format json` event stream.
/// Non-JSON noise lines (opencode logs a few to stdout) are ignored. A stream
/// in steps answers with its final step only (see [`super::opencodesteps`]).
/// If the stream carries only error events, their messages are returned so the
/// broker logs a clean explanation instead of silence. Events with neither
/// text nor an error are told in one line; the raw stdout is the reply only
/// when none of it was an event.
pub fn opencode_json(raw: &str) -> String {
    let events: Vec<Value> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_str(l).ok()) // skip pino-style log noise
        .collect();
    let is_error = |v: &&Value| v.get("type").and_then(Value::as_str) == Some("error");
    let reply = if has_steps(&events) {
        final_step_text(&events)
    } else {
        every_text(events.iter().filter(|v| !is_error(v)))
    };
    if let Some(reply) = reply {
        return reply;
    }
    let errors: Vec<String> = events.iter().filter(is_error).map(error_message).collect();
    if !errors.is_empty() {
        return format!("[opencode error] {}", errors.join("; "));
    }
    silent_summary(&events).unwrap_or_else(|| raw.trim().to_string())
}

/// The pre-step reading: every string under a `"text"` key, joined. Kept for
/// streams with no `step_start`, where nothing marks narration apart.
fn every_text<'a>(events: impl Iterator<Item = &'a Value>) -> Option<String> {
    let mut texts: Vec<String> = Vec::new();
    events.for_each(|v| collect_text(v, &mut texts));
    (!texts.is_empty()).then(|| texts.join("").trim().to_string())
}

/// Pull a human-readable message out of an opencode `{"type":"error",...}` event.
fn error_message(v: &Value) -> String {
    let err = v.get("error");
    err.and_then(|e| e.get("data"))
        .and_then(|d| d.get("message"))
        .and_then(Value::as_str)
        .or_else(|| err.and_then(|e| e.get("name")).and_then(Value::as_str))
        .unwrap_or("unknown error")
        .to_string()
}

/// Recursively collect strings stored under a `"text"` key — opencode carries
/// assistant output in `{"type":"text","text":...}` parts nested in events.
fn collect_text(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if let Some(Value::String(t)) = map.get("text") {
                if !t.is_empty() {
                    out.push(t.clone());
                }
            }
            for (k, child) in map {
                if k != "text" {
                    collect_text(child, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|c| collect_text(c, out)),
        _ => {}
    }
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod tests;
