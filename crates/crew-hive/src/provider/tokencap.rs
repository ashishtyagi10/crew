//! The name OpenAI's own API wants for a reply's token ceiling.
//!
//! crew speaks one chat-completions wire to OpenRouter, DashScope, NVIDIA,
//! DeepSeek, Gemini and OpenAI, and sent the ceiling as `max_tokens`
//! everywhere. OpenAI's reasoning models (the GPT-5 family, crew's default
//! there) refuse that name outright (`Unsupported parameter: 'max_tokens' …
//! Use 'max_completion_tokens' instead`), so an OpenAI key's first call
//! failed. Every current OpenAI chat model takes `max_completion_tokens`, so
//! on OpenAI's host the field is renamed; every other host still gets the
//! `max_tokens` it documents.

/// `body` with `max_tokens` renamed to `max_completion_tokens` when
/// `endpoint` is OpenAI's own API; any other endpoint's body is untouched.
pub(crate) fn for_endpoint(endpoint: &str, mut body: serde_json::Value) -> serde_json::Value {
    if is_openai(endpoint) {
        if let Some(obj) = body.as_object_mut() {
            if let Some(cap) = obj.remove("max_tokens") {
                obj.insert("max_completion_tokens".into(), cap);
            }
        }
    }
    body
}

/// OpenAI's own host — not a gateway that merely speaks its wire.
fn is_openai(endpoint: &str) -> bool {
    endpoint
        .split("://")
        .nth(1)
        .and_then(|rest| rest.split(['/', ':']).next())
        .is_some_and(|host| host.eq_ignore_ascii_case("api.openai.com"))
}

#[cfg(test)]
#[path = "tokencap_tests.rs"]
mod tests;
