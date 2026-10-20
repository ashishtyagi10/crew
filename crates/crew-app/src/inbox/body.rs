//! An opened mail's text, out of the MIME tree Gmail returns.
//!
//! `messages.get` with `format=full` sends the message as nested parts, each
//! with a `mimeType` and its bytes base64url-encoded in `body.data`. The pane
//! shows words, never markup: the `text/plain` part when the mail has one,
//! else the `text/html` part read as text (`crew_plugin::html`), else a line
//! saying what kind of mail it is. Attachments are named, not fetched.
use base64::Engine;
use serde_json::Value;

/// The text to show for one message, and the names of its attachments.
pub(crate) fn text(msg: &Value) -> (String, Vec<String>) {
    let payload = &msg["payload"];
    let mut files = Vec::new();
    attachments(payload, &mut files);
    let text = find(payload, "text/plain")
        .map(|t| clean(&t))
        .or_else(|| find(payload, "text/html").map(|h| clean(&crew_plugin::html::readable(&h))))
        .unwrap_or_else(|| {
            let kind = payload["mimeType"].as_str().unwrap_or("unknown");
            format!("(no text in this mail \u{2014} it is {kind}; o opens it in the browser)")
        });
    (text, files)
}

/// The first part of `mime` in the tree, depth first, decoded.
fn find(part: &Value, mime: &str) -> Option<String> {
    let named_file = part["filename"].as_str().is_some_and(|f| !f.is_empty());
    if part["mimeType"].as_str() == Some(mime) && !named_file {
        if let Some(text) = part["body"]["data"].as_str().and_then(decode) {
            return Some(text);
        }
    }
    part["parts"].as_array()?.iter().find_map(|p| find(p, mime))
}

fn attachments(part: &Value, out: &mut Vec<String>) {
    if let Some(name) = part["filename"].as_str().filter(|f| !f.is_empty()) {
        out.push(name.to_string());
    }
    for p in part["parts"].as_array().into_iter().flatten() {
        attachments(p, out);
    }
}

/// base64url, padded or not, as UTF-8 (lossy: a stray byte is not a reason to
/// show nothing).
pub(crate) fn decode(data: &str) -> Option<String> {
    let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(clean.trim_end_matches('='))
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Line endings made `\n`, control characters out (a mail's text reaches the
/// grid, and an escape sequence in it must not), blank runs folded to one.
pub(crate) fn clean(text: &str) -> String {
    let unix = text.replace("\r\n", "\n").replace('\r', "\n");
    let printable: String = unix
        .chars()
        .filter(|&c| c == '\n' || c == '\t' || !c.is_control())
        .collect();
    let mut out: Vec<&str> = Vec::new();
    for line in printable.lines().map(str::trim_end) {
        if !(line.is_empty() && out.last().is_some_and(|l| l.is_empty())) {
            out.push(line);
        }
    }
    out.join("\n").trim().to_string()
}

#[cfg(test)]
#[path = "body_tests.rs"]
mod tests;
