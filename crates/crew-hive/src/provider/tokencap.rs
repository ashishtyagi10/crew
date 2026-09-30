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

#[path = "stopseq.rs"]
mod stopseq;

/// `body` as `endpoint` takes it: `max_tokens` renamed to
/// `max_completion_tokens` on OpenAI's own API, and everywhere else a
/// text-protocol request stopped at `</tool_call>` ([`stopseq`]) — the one
/// place every chat-completions body passes on its way to a host.
pub(crate) fn for_endpoint(endpoint: &str, mut body: serde_json::Value) -> serde_json::Value {
    let openai = is_openai(endpoint);
    if openai {
        if let Some(obj) = body.as_object_mut() {
            if let Some(cap) = obj.remove("max_tokens") {
                obj.insert("max_completion_tokens".into(), cap);
            }
        }
    }
    stopseq::end_text_calls(openai, &mut body);
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
