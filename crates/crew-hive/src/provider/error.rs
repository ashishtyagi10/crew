//! A provider's failure, and the sentence a person is shown for it.

use super::hint;

#[derive(Debug)]
pub enum ProviderError {
    Http(String),
    Decode(String),
    Api(String),
    /// An answer with no provider sentence in it (a proxy's HTML error page,
    /// an empty body, a 2xx that is not JSON), said as its status and host:
    /// `HTTP 502 from dashscope-intl.aliyuncs.com: Bad Gateway` (`status`).
    Status(String),
    /// No key for the named variable. It carries the name because
    /// `OpenRouterProvider` backs six providers through different variables —
    /// a fixed string here named `ANTHROPIC_API_KEY` for all of them.
    MissingKey(&'static str),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderError::Http(s) => write!(f, "http error: {s}"),
            ProviderError::Decode(s) => write!(f, "decode error: {s}"),
            ProviderError::Api(s) => write!(f, "{}", api_message(s)),
            ProviderError::Status(s) => write!(f, "{}", hint::told("", s, "")),
            ProviderError::MissingKey(var) => write!(f, "{var} not set"),
        }
    }
}

impl std::error::Error for ProviderError {}

/// A provider's API error, in a sentence rather than a JSON blob.
///
/// Providers answer failures with a JSON envelope, and it used to reach the
/// user verbatim — pasted into the chat and truncated mid-structure, so the
/// one useful sentence inside it arrived cut in half. The message field is
/// what a person needs; the envelope is what a debugger needs, and a
/// debugger can read the log.
///
/// An error the user can fix additionally gets told what to DO (`hint`): a
/// rejected key first, the most common provider error there is and the only
/// one the user can always fix, then a spent rate limit, an empty account,
/// a thread past the model's context, and a model the host does not serve.
pub fn api_message(body: &str) -> String {
    let msg = extract_message(body).unwrap_or_else(|| one_line(body));
    hint::told("api error: ", &msg, body)
}

/// The `error.message` string from a provider's JSON envelope, if it has one.
/// Hand-rolled rather than a serde type: every provider nests it slightly
/// differently, and a parse failure here must degrade to showing the body,
/// never to hiding the error.
pub(super) fn extract_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let msg = v
        .get("error")
        .and_then(|e| e.get("message"))
        .or_else(|| v.get("message"))?
        .as_str()?;
    (!msg.trim().is_empty()).then(|| one_line(msg))
}

/// Whitespace collapsed to single spaces and trimmed to something a status
/// line can hold — a multi-line body must not become a multi-line error.
pub(super) fn one_line(s: &str) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.chars().take(200).collect()
}

#[cfg(test)]
#[path = "apimessage_tests.rs"]
mod api_message_tests;
