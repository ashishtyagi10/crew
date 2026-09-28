//! What a response says about its own bill: which model answered, and how
//! much of the prompt the provider served from its cache.
//!
//! WHY: a reply was priced at the model the REQUEST named, with every input
//! token at the full rate. Neither held. A fallback chain answers with its
//! second model when the first 404s, and both DashScope (implicit cache, on
//! by default and impossible to turn off) and Anthropic bill cached prompt
//! tokens at a fraction of the input price — a long tool loop re-sends the
//! same prefix every round, so most of its input is exactly that fraction.
//! Each provider spells these fields differently; the readers are here, once.
use serde_json::Value;

use super::Completion;

/// A token count off the wire: absent, null or negative is 0, and a count
/// past `u32` saturates rather than wraps.
pub(crate) fn count(v: &Value) -> u32 {
    v.as_u64().unwrap_or(0).min(u64::from(u32::MAX)) as u32
}

/// `(read, written)` cache tokens from an OpenAI-shape
/// `usage.prompt_tokens_details`. `cached_tokens` is what OpenAI and
/// DashScope's implicit cache report, and it is PART of `prompt_tokens`;
/// DashScope's explicit cache adds `cache_creation_input_tokens` beside it.
pub(crate) fn openai_cache(details: &Value) -> (u32, u32) {
    (
        count(&details["cached_tokens"]),
        count(&details["cache_creation_input_tokens"]),
    )
}

/// `(read, written)` cache tokens from an Anthropic `usage` object. Unlike
/// the OpenAI shape these are NOT inside `input_tokens` — Anthropic's
/// `input_tokens` is the uncached remainder only — so [`anthropic_input`]
/// adds them back to get the whole prompt.
pub(crate) fn anthropic_cache(usage: &Value) -> (u32, u32) {
    (
        count(&usage["cache_read_input_tokens"]),
        count(&usage["cache_creation_input_tokens"]),
    )
}

/// The whole prompt an Anthropic `usage` object describes: the uncached
/// remainder plus what was read from and written to the cache. Kept whole
/// so `input_tokens` means the same thing on every provider — the context
/// fill the pane shows — and the cached part is a subset of it.
pub(crate) fn anthropic_input(usage: &Value) -> u32 {
    let (read, written) = anthropic_cache(usage);
    count(&usage["input_tokens"])
        .saturating_add(read)
        .saturating_add(written)
}

/// The `model` a response body names, or `""` when it names none.
pub(crate) fn model_of(body: &Value) -> String {
    body["model"].as_str().unwrap_or("").trim().to_string()
}

impl Completion {
    /// `self`, answered by `model` unless the response already said which
    /// model it was — its own word outranks the attempt's, since a router
    /// may serve a slug with a model the slug does not spell.
    pub(crate) fn served_by(mut self, model: &str) -> Self {
        if self.model.is_empty() {
            self.model = model.to_string();
        }
        self
    }
}

#[cfg(test)]
#[path = "served_tests.rs"]
mod tests;
