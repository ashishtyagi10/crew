//! Tick pacing for mid-reply StatsTick emission: a pure rate gate (clock is
//! a parameter, mirroring `Tasks::attach(.., now)`) — at most one tick per
//! agent per TICK_GAP_MS, enforced by the caller holding `last_ms`.
use crate::PluginEvent;

/// Minimum gap between two StatsTicks for one agent.
pub(crate) const TICK_GAP_MS: u64 = 150;

/// First tick always passes; later ticks pass once `min_gap_ms` elapsed.
pub(crate) fn should_tick(last_ms: Option<u64>, now_ms: u64, min_gap_ms: u64) -> bool {
    last_ms.is_none_or(|l| now_ms.saturating_sub(l) >= min_gap_ms)
}

/// Build a fresh, rate-limited `on_tokens` callback for one agent hop: it
/// converts a running token estimate into `PluginEvent::StatsTick` events
/// through `tick_emit`, but only when the estimate grew AND at most once per
/// `TICK_GAP_MS` (first call always passes). Each call to this function opens
/// its own clock (`Instant::now()` at hop start) and its own `last_ms`/
/// `last_value` state, so concurrent or successive hops rate-limit
/// independently of one another.
pub(crate) fn hop_ticker(
    tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    agent: String,
) -> std::sync::Arc<dyn Fn(u64) + Send + Sync> {
    let last_tick_ms = std::sync::Mutex::new(None::<u64>);
    let last_value = std::sync::Mutex::new(0u64);
    let hop_start = std::time::Instant::now();
    std::sync::Arc::new(move |tokens: u64| {
        let now_ms = hop_start.elapsed().as_millis() as u64;
        let mut last = last_tick_ms.lock().unwrap_or_else(|e| e.into_inner());
        let mut val = last_value.lock().unwrap_or_else(|e| e.into_inner());
        if tokens > *val && should_tick(*last, now_ms, TICK_GAP_MS) {
            *last = Some(now_ms);
            *val = tokens;
            tick_emit(PluginEvent::StatsTick {
                agent: agent.clone(),
                tokens,
            });
        }
    })
}

/// A tick emitter that discards every `StatsTick` — for call paths that never
/// dial an agent (quick constructs) or tests that don't care about ticking.
pub(crate) fn noop_tick_emit() -> std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync> {
    std::sync::Arc::new(|_| {})
}

/// Minimum gap between two Delta flushes for one agent. Tighter than
/// `TICK_GAP_MS`: text is what the eye tracks and wants to feel continuous,
/// while a token counter reads fine at 150ms.
pub(crate) const TEXT_GAP_MS: u64 = 80;

/// Whether streamed text is forwarded at all. `CREW_STREAM_TEXT=0` turns
/// every gate into a no-op, restoring the pre-streaming behaviour for a
/// regressed run or a deterministic test.
pub(crate) fn text_streaming_enabled() -> bool {
    !matches!(std::env::var("CREW_STREAM_TEXT").as_deref(), Ok("0"))
}

/// Coalescing buffer for ONE agent's streamed text, so a provider emitting
/// per-token fragments cannot flood the host (the app polls plugin events
/// synchronously on the winit thread).
///
/// Unlike [`should_tick`]'s numeric gate, a fragment arriving inside the gap
/// is BUFFERED, never dropped: a token estimate is monotonic, so a skipped
/// tick is corrected by the next one, but text is cumulative and a dropped
/// fragment would stay missing until the settled `Message` replaced the whole
/// reply. `now_ms` is a parameter rather than a clock read, so this stays a
/// pure, testable gate — the same convention as `should_tick`.
pub(crate) struct TextGate {
    buf: String,
    last_ms: Option<u64>,
}

impl TextGate {
    pub(crate) fn new() -> Self {
        Self {
            buf: String::new(),
            last_ms: None,
        }
    }

    /// Buffer `text`, returning the payload to send as one `Delta` when the
    /// gap has elapsed (the first non-empty push always passes).
    pub(crate) fn push(&mut self, text: &str, now_ms: u64) -> Option<String> {
        self.buf.push_str(text);
        // An empty buffer must not consume the first-flush allowance, or a
        // provider's empty keep-alive frame would delay the first real text.
        if self.buf.is_empty() || !should_tick(self.last_ms, now_ms, TEXT_GAP_MS) {
            return None;
        }
        self.last_ms = Some(now_ms);
        Some(std::mem::take(&mut self.buf))
    }
}

/// Build a rate-limited `on_text` callback for one agent hop: fragments go
/// through a [`TextGate`] and each flush is emitted as a `PluginEvent::Delta`
/// via `tick_emit`. Own clock and own state per call, exactly like
/// [`hop_ticker`], so concurrent hops gate independently.
///
/// There is deliberately NO end-of-hop flush: the gate may swallow the final
/// fragment, and that is fine — deltas are advisory and the settled `Message`
/// carries the full reply milliseconds later. Skipping the flush keeps this a
/// pure function of its inputs with no lifecycle to get wrong.
pub(crate) fn hop_texter(
    tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    agent: String,
) -> std::sync::Arc<dyn Fn(&str) + Send + Sync> {
    hop_texter_with(tick_emit, agent, text_streaming_enabled())
}

/// [`hop_texter`] with the switch handed in rather than read from the
/// environment.
///
/// `CREW_STREAM_TEXT` is process-global, and the two tests that exercised the
/// off state used to SET it — serialised against each other by a lock, and
/// against nothing else. Every other test in the process that built a texter
/// and expected a `Delta` was racing them, which is exactly the flake that
/// turned up here: `run_tools_follow_up_dial_reports_usage_and_emits_ticks`
/// failing in a full run and never in isolation. A test that needs to know
/// something about the world should be told it, not made to look.
pub(crate) fn hop_texter_with(
    tick_emit: std::sync::Arc<dyn Fn(PluginEvent) + Send + Sync>,
    agent: String,
    enabled: bool,
) -> std::sync::Arc<dyn Fn(&str) + Send + Sync> {
    let gate = std::sync::Mutex::new(TextGate::new());
    let hop_start = std::time::Instant::now();
    std::sync::Arc::new(move |text: &str| {
        if !enabled {
            return;
        }
        let now_ms = hop_start.elapsed().as_millis() as u64;
        let mut g = gate.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(payload) = g.push(text, now_ms) {
            tick_emit(PluginEvent::Delta {
                agent: agent.clone(),
                text: payload,
            });
        }
    })
}

#[cfg(test)]
#[path = "tick_tests.rs"]
mod tests;
