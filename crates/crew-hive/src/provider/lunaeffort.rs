//! GPT-6 Luna takes function tools on Chat Completions only at no effort.
//!
//! OpenAI names `gpt-6-luna` as the replacement for `gpt-4.1`, which it shuts
//! down 2027-01-14 (deprecations page, read 2026-11-23), so Luna is the
//! `openai` chain's last rung. Its model page says "Chat Completions supports
//! function calling only with `reasoning_effort` set to `none`" and that the
//! default effort is `medium` — so a tool request sent the way every other
//! OpenAI model takes it would be refused, and crew's agent sends tools on
//! every turn. A request that carries tools to Luna on OpenAI's own host is
//! therefore told `reasoning_effort: none`; nothing else is touched. Sol's
//! page carries the same line, but no chain names Sol, so it is left alone
//! rather than guessed at.
use serde_json::{json, Value};

/// Adds `reasoning_effort: "none"` to a tool request for `gpt-6-luna` (or a
/// dated snapshot of it) bound for OpenAI's own host, unless one is set.
pub(super) fn tools_at_no_effort(openai: bool, body: &mut Value) {
    let luna = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|m| m == "gpt-6-luna" || m.starts_with("gpt-6-luna-"));
    let tools = body
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|t| !t.is_empty());
    if !(openai && luna && tools) || body.get("reasoning_effort").is_some() {
        return;
    }
    if let Some(obj) = body.as_object_mut() {
        obj.insert("reasoning_effort".into(), json!("none"));
    }
}

#[cfg(test)]
#[path = "lunaeffort_tests.rs"]
mod tests;
