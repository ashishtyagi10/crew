//! Asking an OpenAI-compatible endpoint to SHOW its reasoning.
//!
//! Every host parses whatever reasoning comes back (`openai_http`); only two
//! need asking. DashScope's Qwen3 hides its thinking unless the request
//! carries `enable_thinking: true` — and rejects that flag on a non-streamed
//! request outright, so it rides the streaming body alone. OpenRouter's
//! unified field is `reasoning: {enabled: true}`, which it translates for
//! whichever upstream serves the slug. Anything else (NVIDIA NIM, a local
//! vLLM, the DIRECT rows) gets no extra field: an unknown key is a 400 on
//! some of them, and their models reason in the open already.
//!
//! `CREW_THINKING=0` turns the opt-in off, in the style of `CREW_STREAM_TEXT`
//! (the broker's text-streaming switch): a run that regresses on a provider's
//! reasoning field has one knob to turn, and a test one value to pass.

/// Whether requests may opt into reasoning at all. Default on; only an
/// explicit `0` turns it off.
pub(crate) fn thinking_enabled() -> bool {
    enabled_from(std::env::var("CREW_THINKING").ok().as_deref())
}

/// [`thinking_enabled`]'s decision, with the value passed in — the testable
/// half, in the manner of `AnthropicProvider::from_key`.
pub(crate) fn enabled_from(var: Option<&str>) -> bool {
    !matches!(var, Some("0"))
}

/// The body field this endpoint needs to show its reasoning, if any —
/// `(key, value)` — for a request that streams (`streaming`) or does not.
pub(crate) fn opt_in(endpoint: &str, streaming: bool) -> Option<(&'static str, serde_json::Value)> {
    opt_in_if(thinking_enabled(), endpoint, streaming)
}

/// [`opt_in`] with the switch handed in rather than read from the environment.
pub(crate) fn opt_in_if(
    enabled: bool,
    endpoint: &str,
    streaming: bool,
) -> Option<(&'static str, serde_json::Value)> {
    if !enabled {
        return None;
    }
    if endpoint.contains("dashscope") {
        return streaming.then_some(("enable_thinking", serde_json::json!(true)));
    }
    if endpoint.contains("openrouter.ai") {
        return Some(("reasoning", serde_json::json!({"enabled": true})));
    }
    None
}

/// Remove any reasoning opt-in from `body`, for the one retry after a 400.
/// `true` when there was one to remove — a body that never asked has nothing
/// to retry without.
pub(crate) fn strip(body: &mut serde_json::Value) -> bool {
    let Some(obj) = body.as_object_mut() else {
        return false;
    };
    let had = obj.remove("enable_thinking").is_some();
    obj.remove("reasoning").is_some() || had
}

#[cfg(test)]
#[path = "thinking_tests.rs"]
mod tests;
