//! Anthropic's prompt cache, asked for when the prefix will be sent again.
//!
//! A native tool loop (`apiagent::native`) resends the whole conversation
//! every round — system, tools, the opening prompt and every turn since — so
//! round N paid the full input price, and the full prefill wait, for all
//! that rounds 1..N-1 had already sent. One TOP-LEVEL field,
//! `"cache_control": {"type": "ephemeral"}`, turns on Anthropic's automatic
//! caching: the API sets the breakpoint on the last cacheable block and moves
//! it forward as the conversation grows, so the next round reads the shared
//! prefix back at a tenth of the input rate. Default TTL five minutes, no
//! beta header.
//!
//! Not on every request. A cache WRITE costs 1.25× input, and a request whose
//! prefix is never sent again only ever writes: a pure 25% surcharge. So only
//! a request that WILL be followed by one sharing its prefix asks — one
//! carrying tools, whose next round replays it. A one-shot — the router, a
//! judge, the closing `lastword` answer, a relay prompt rebuilt from scratch
//! each round, and the broker's one-time cutoff continuation (turns but no
//! tools, never followed) — does not. A
//! prefix under the model's minimum (512 tokens on the 5.x models, 4096 on
//! Haiku 4.5) is silently not cached: no error, no charge.
//!
//! `CREW_ANTHROPIC_CACHE=0` turns the asking off, in the style of
//! `CREW_THINKING`: for a gateway behind `ANTHROPIC_BASE_URL` that refuses
//! the field, or an uncached bill to compare against.
use super::CompletionRequest;

/// Whether requests may ask for the cache at all. Default on; only an
/// explicit `0` turns it off.
pub(crate) fn enabled() -> bool {
    enabled_from(std::env::var("CREW_ANTHROPIC_CACHE").ok().as_deref())
}

/// [`enabled`]'s decision, with the value passed in — the testable half, in
/// the manner of `thinking::enabled_from`.
pub(crate) fn enabled_from(var: Option<&str>) -> bool {
    !matches!(var, Some("0"))
}

/// Whether `req` should ask for the cache: the switch is on and it is a tool
/// loop's round, so a request sharing its prefix will follow. Turns alone do
/// not count: the only tool-free request with turns is the cutoff
/// continuation, sent once, which would pay the write and never read it.
pub(crate) fn caches(enabled: bool, req: &CompletionRequest) -> bool {
    enabled && !req.tools.is_empty()
}

/// The top-level field that asks for automatic caching.
pub(crate) fn field() -> serde_json::Value {
    serde_json::json!({"type": "ephemeral"})
}

#[cfg(test)]
#[path = "anthropiccache_tests.rs"]
mod tests;
