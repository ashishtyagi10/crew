//! Why a reply stopped, reduced to the one answer crew acts on: did it run
//! into the output-token ceiling ([`super::Completion::truncated`]).
//!
//! No provider used to read it. A reply cut at `max_tokens` arrived looking
//! finished — an answer ending mid-sentence was shown as the answer, and a
//! `sys:write_file` whose JSON stopped halfway failed as "not valid JSON"
//! with nothing saying the budget, not the model, ended it. The two wire
//! vocabularies sit side by side here so every provider maps onto one word.

/// OpenAI-compatible `finish_reason` (OpenRouter, DashScope, NIM, …):
/// `"length"` is the ceiling; `"stop"`, `"tool_calls"` and the rest are the
/// model finishing on its own.
pub(super) fn openai(finish_reason: Option<&str>) -> bool {
    finish_reason == Some("length")
}

/// Anthropic `stop_reason` — the Messages API, its streamed `message_delta`,
/// and the Claude Code CLI's closing `result` line all spell it the same way.
pub(super) fn anthropic(stop_reason: Option<&str>) -> bool {
    stop_reason == Some("max_tokens")
}

/// The model declined the request rather than answering it: Anthropic's
/// `refusal` stop reason, the OpenAI shape's `content_filter` finish. Read
/// as nothing more, it was an empty reply — asked again twice (`resume`),
/// declined twice more, then shown as no answer with no reason.
pub(super) fn refused(reason: Option<&str>) -> bool {
    matches!(reason, Some("refusal" | "content_filter"))
}

/// What a declined request ends as: the same request is declined again, so
/// it is never retried, and the turn says why it has no answer.
pub(super) const DECLINED: &str =
    "the model declined to answer this request (its stop reason was a refusal)";

#[cfg(test)]
#[path = "stopreason_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stopreason_wire_tests.rs"]
mod wire_tests;

#[cfg(test)]
#[path = "stoprefused_tests.rs"]
mod refused_tests;
