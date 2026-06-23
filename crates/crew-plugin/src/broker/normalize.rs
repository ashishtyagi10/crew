//! Turning an agent CLI's raw stdout into a clean reply string. `claude -p
//! --output-format text` and `codex exec` already print just the reply on
//! stdout (codex's session banner goes to stderr, which the runner discards),
//! so they use [`super::Normalize::Raw`]. opencode emits a stream of JSON event
//! lines; [`opencode_json`] pulls the assistant text out and surfaces errors.
use serde_json::Value;

/// Extract the assistant's reply from opencode's `--format json` event stream.
/// Non-JSON noise lines (opencode logs a few to stdout) are ignored. If the
/// stream carries only error events, their messages are returned so the broker
/// logs a clean explanation instead of silence.
pub fn opencode_json(raw: &str) -> String {
    let mut texts: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue; // pino-style log noise, not an event
        };
        if v.get("type").and_then(Value::as_str) == Some("error") {
            errors.push(error_message(&v));
        } else {
            collect_text(&v, &mut texts);
        }
    }
    if !texts.is_empty() {
        return texts.join("").trim().to_string();
    }
    if !errors.is_empty() {
        return format!("[opencode error] {}", errors.join("; "));
    }
    raw.trim().to_string()
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
