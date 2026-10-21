//! An opened mail's words, out of the MIME tree Gmail returns.
//!
//! `messages.get` with `format=full` sends the message as nested parts, each
//! with a `mimeType`, its headers, and its bytes base64url-encoded in
//! `body.data`. The bytes are read in the charset the part's `Content-Type`
//! names (`charset`), and become a [`Rich`]: the `text/html` part drawn as
//! markdown when there is one, the `text/plain` part as its other version
//! (`v`), else a line saying what kind of mail it is. Attachments are named,
//! not fetched.
use base64::Engine;
use serde_json::Value;

use super::rich::Rich;

/// The message to show, and the names of its attachments.
pub(crate) fn text(msg: &Value) -> (Rich, Vec<String>) {
    let payload = &msg["payload"];
    let mut files = Vec::new();
    attachments(payload, &mut files);
    let html = find(payload, "text/html").map(|(t, cs)| Rich::of_html(&t, cs));
    let plain = find(payload, "text/plain").map(|(t, cs)| Rich::of_plain(&clean(&t), cs));
    let rich = match (html, plain) {
        // HTML that reads as nothing (one big picture) loses to its words.
        (Some(h), Some(p)) if h.body.trim().is_empty() && h.quote.is_none() => Rich {
            alt: Some(Box::new(h)),
            ..p
        },
        (Some(h), Some(p)) => Rich {
            alt: Some(Box::new(p)),
            ..h
        },
        (Some(one), None) | (None, Some(one)) => one,
        (None, None) => {
            let kind = payload["mimeType"].as_str().unwrap_or("unknown");
            Rich::of_plain(
                &format!("(no text in this mail \u{2014} it is {kind}; o opens it in the browser)"),
                None,
            )
        }
    };
    (rich, files)
}

/// The first part of `mime` in the tree, depth first: its text, and the
/// charset it was in when that is not one crew reads.
pub(crate) fn find(part: &Value, mime: &str) -> Option<(String, Option<String>)> {
    let named_file = part["filename"].as_str().is_some_and(|f| !f.is_empty());
    if part["mimeType"].as_str() == Some(mime) && !named_file {
        if let Some(bytes) = part["body"]["data"].as_str().and_then(bytes) {
            let cs = super::charset::of(&header(part, "Content-Type"));
            return Some(super::charset::decode(&bytes, cs.as_deref()));
        }
    }
    part["parts"].as_array()?.iter().find_map(|p| find(p, mime))
}

/// A part's own header (`payload.headers`, or a sub-part's `headers`).
fn header(part: &Value, name: &str) -> String {
    part["headers"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|h| {
            h["name"]
                .as_str()
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
        .and_then(|h| h["value"].as_str())
        .unwrap_or("")
        .to_string()
}

/// A part a picture can come from: the name `cid:` gives it, and where its
/// bytes are (inline, or an attachment to fetch).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Part {
    pub cid: Option<String>,
    pub name: String,
    pub mime: String,
    pub att: Option<String>,
    pub data: Option<String>,
    pub size: u64,
}

/// Every part sent as a file (it has a name): what the reader lists under
/// its message, to be saved and opened.
pub(crate) fn files(msg: &Value) -> Vec<Part> {
    fn walk(p: &Value, out: &mut Vec<Part>) {
        if let Some(name) = p["filename"].as_str().filter(|f| !f.is_empty()) {
            out.push(Part {
                cid: None,
                name: name.to_string(),
                mime: p["mimeType"].as_str().unwrap_or("").to_ascii_lowercase(),
                att: p["body"]["attachmentId"].as_str().map(str::to_string),
                data: p["body"]["data"].as_str().map(str::to_string),
                size: p["body"]["size"].as_u64().unwrap_or(0),
            });
        }
        for k in p["parts"].as_array().into_iter().flatten() {
            walk(k, out);
        }
    }
    let mut out = Vec::new();
    walk(&msg["payload"], &mut out);
    out
}

/// Every part that is a picture, or that a `cid:` could name.
pub(crate) fn pictures(msg: &Value) -> Vec<Part> {
    fn walk(p: &Value, out: &mut Vec<Part>) {
        let mime = p["mimeType"].as_str().unwrap_or("").to_ascii_lowercase();
        let cid = header(p, "Content-ID");
        let cid = cid
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .trim();
        if mime.starts_with("image/") || !cid.is_empty() {
            out.push(Part {
                cid: (!cid.is_empty()).then(|| cid.to_string()),
                name: p["filename"].as_str().unwrap_or("").to_string(),
                mime,
                att: p["body"]["attachmentId"].as_str().map(str::to_string),
                data: p["body"]["data"].as_str().map(str::to_string),
                size: p["body"]["size"].as_u64().unwrap_or(0),
            });
        }
        for k in p["parts"].as_array().into_iter().flatten() {
            walk(k, out);
        }
    }
    let mut out = Vec::new();
    walk(&msg["payload"], &mut out);
    out
}

fn attachments(part: &Value, out: &mut Vec<String>) {
    if let Some(name) = part["filename"].as_str().filter(|f| !f.is_empty()) {
        out.push(name.to_string());
    }
    for p in part["parts"].as_array().into_iter().flatten() {
        attachments(p, out);
    }
}

/// base64url, padded or not.
pub(crate) fn bytes(data: &str) -> Option<Vec<u8>> {
    let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(clean.trim_end_matches('='))
        .ok()
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
