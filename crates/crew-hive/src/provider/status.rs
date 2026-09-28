//! What a failed HTTP answer SAYS.
//!
//! Every provider body went straight to the JSON parser, whatever the status.
//! A 502 or 503 from the proxy in front of a model host is an HTML page, so
//! the person read `decode error: expected value at line 1 column 1`: a
//! sentence about crew's parser, naming neither the status nor the host,
//! that read like a crew bug. [`settle`] is the one gate an answer passes on
//! its way to a parser. A 2xx JSON body is parsed; a JSON error envelope
//! keeps the provider's own sentence, which [`super::api_message`] reads;
//! anything else is said as what it is: `HTTP 502 from
//! dashscope-intl.aliyuncs.com: Bad Gateway`.
use super::{one_line, wire::host_of, Completion, ProviderError};

/// How long the status sentence may run: one line on an error card.
const MAX: usize = 120;

/// The reply in an answer, or the error it stands for. `parse` sees only a
/// 2xx body that is JSON; it still finds a 2xx that carries an `error`.
pub(super) fn settle(
    status: u16,
    endpoint: &str,
    body: &str,
    parse: impl FnOnce(&str) -> Result<Completion, ProviderError>,
) -> Result<Completion, ProviderError> {
    let ok = (200..300).contains(&status);
    if ok && serde_json::from_str::<serde::de::IgnoredAny>(body).is_ok() {
        return parse(body);
    }
    Err(failure(status, endpoint, body))
}

/// The error an answer that is not a reply stands for. A JSON envelope with
/// a message is the provider speaking, and its sentence is the diagnosis;
/// anything else has only its status, its host and its page title to say.
pub(super) fn failure(status: u16, endpoint: &str, body: &str) -> ProviderError {
    if super::extract_message(body).is_some() {
        return ProviderError::Api(body.to_string());
    }
    ProviderError::Status(sentence(status, host_of(endpoint), body))
}

/// `HTTP <status> from <host>: <reason>`, then the page's title (or, with no
/// title, its text) when that adds anything, cut to [`MAX`] characters.
fn sentence(status: u16, host: &str, body: &str) -> String {
    let empty = body.trim().is_empty();
    let reason = match status {
        200..=299 if empty => Some("the reply is empty"),
        200..=299 => Some("the reply is not JSON"),
        _ => reqwest::StatusCode::from_u16(status)
            .ok()
            .and_then(|s| s.canonical_reason()),
    };
    let mut s = format!("HTTP {status} from {host}");
    if let Some(r) = reason {
        s.push_str(": ");
        s.push_str(r);
    }
    let detail = title(body).unwrap_or_else(|| one_line(&untagged(body)));
    // "502 Bad Gateway" under "Bad Gateway" says nothing new.
    let news = detail.replace(&status.to_string(), "");
    let news = news.trim_matches(|c: char| c.is_whitespace() || ":-|".contains(c));
    if !news.is_empty() && !reason.is_some_and(|r| r.eq_ignore_ascii_case(news)) {
        s.push_str(if reason.is_some() { " \u{2014} " } else { ": " });
        s.push_str(&detail);
    }
    clip(s)
}

/// The text of the page's `<title>`, when it has a non-empty one.
fn title(body: &str) -> Option<String> {
    // ASCII lowercasing keeps every byte offset, so indices carry over.
    let low = body.to_ascii_lowercase();
    let open = low.find("<title")?;
    let start = open + low[open..].find('>')? + 1;
    let end = start + low[start..].find("</title")?;
    let t = one_line(&untagged(&body[start..end]));
    (!t.is_empty()).then_some(t)
}

/// `s` with every `<…>` tag replaced by a space, so words either side of
/// one stay apart.
fn untagged(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut inside = false;
    for c in s.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

/// At most [`MAX`] characters, the last one an ellipsis when it was cut.
fn clip(s: String) -> String {
    if s.chars().count() <= MAX {
        return s;
    }
    let mut cut: String = s.chars().take(MAX - 1).collect();
    cut.push('\u{2026}');
    cut
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
