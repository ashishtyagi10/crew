//! A `tools/call` result as the words an agent reads.
//!
//! Only `text` blocks were kept. An MCP tool may answer with an `image`, an
//! `audio` clip, a `resource_link` (a file or page it found), an embedded
//! `resource`, or `structuredContent` beside — or instead of — its text, and
//! each of those reached the agent as an empty result: a search that found
//! three files read as having found nothing. Every block now says what it is;
//! binary bytes are described, never pasted.
use serde_json::Value;

/// The result's content, one block a line; `structuredContent` as its JSON
/// when no block carries words.
pub(crate) fn text_of(r: &Value) -> String {
    let blocks: Vec<String> = r
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(block)
        .collect();
    if blocks.is_empty() {
        if let Some(s) = r.get("structuredContent").filter(|v| !v.is_null()) {
            return s.to_string();
        }
    }
    blocks.join("\n")
}

fn block(c: &Value) -> Option<String> {
    let s = |k: &str| c.get(k).and_then(Value::as_str);
    match s("type")? {
        "text" => s("text").map(str::to_string),
        kind @ ("image" | "audio") => Some(format!(
            "[{kind} {}, {}]",
            s("mimeType").unwrap_or("?"),
            size(s("data").unwrap_or(""))
        )),
        "resource_link" => {
            let name = s("name").or(s("title")).unwrap_or("resource");
            let about = s("description").map_or(String::new(), |d| format!(" \u{2014} {d}"));
            Some(format!("[{name}: {}]{about}", s("uri").unwrap_or("?")))
        }
        "resource" => {
            let r = c.get("resource")?;
            let uri = r.get("uri").and_then(Value::as_str).unwrap_or("?");
            match (
                r.get("text").and_then(Value::as_str),
                r.get("blob").and_then(Value::as_str),
            ) {
                (Some(t), _) => Some(format!("[{uri}]\n{t}")),
                (None, Some(b)) => Some(format!("[{uri}, {} binary]", size(b))),
                (None, None) => Some(format!("[{uri}]")),
            }
        }
        _ => None,
    }
}

/// The decoded size of base64 `data`, as people say it.
fn size(data: &str) -> String {
    let bytes = data.trim_end_matches('=').len() * 3 / 4;
    match bytes {
        b if b >= 1024 * 1024 => format!("{:.1} MB", b as f64 / (1024.0 * 1024.0)),
        b if b >= 1024 => format!("{} KB", b / 1024),
        b => format!("{b} B"),
    }
}

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
