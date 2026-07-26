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
    let gate = std::sync::Mutex::new(TextGate::new());
    let hop_start = std::time::Instant::now();
    let enabled = text_streaming_enabled();
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
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Serialises the two `hop_texter` tests below against each other's
    /// `CREW_STREAM_TEXT` mutation — the var is process-global and Rust runs
    /// tests in parallel, so a bare `set_var`/`remove_var` would race any test
    /// that reads it concurrently. Mirrors `crew-app::envlock::with_home`.
    static STREAM_TEXT_LOCK: Mutex<()> = Mutex::new(());

    type TickEmit = Arc<dyn Fn(PluginEvent) + Send + Sync>;

    #[test]
    fn first_tick_passes_then_gap_enforced() {
        assert!(should_tick(None, 0, TICK_GAP_MS));
        assert!(!should_tick(Some(1000), 1149, TICK_GAP_MS));
        assert!(should_tick(Some(1000), 1150, TICK_GAP_MS));
        assert!(
            !should_tick(Some(1000), 999, TICK_GAP_MS),
            "clock skew saturates, no panic"
        );
    }

    #[test]
    fn text_gate_first_push_flushes_immediately() {
        let mut g = TextGate::new();
        assert_eq!(g.push("hello", 0).as_deref(), Some("hello"));
    }

    #[test]
    fn text_gate_buffers_inside_the_gap_and_loses_no_character() {
        // This is the one way TextGate MUST differ from should_tick's numeric
        // gate: a token estimate is monotonic so a skipped tick self-heals,
        // but text is cumulative — a fragment inside the gap has to be kept.
        let mut g = TextGate::new();
        let mut flushed = String::new();
        for (frag, t) in [("a", 0u64), ("b", 10), ("c", 40), ("d", 70), ("e", 100)] {
            if let Some(p) = g.push(frag, t) {
                flushed.push_str(&p);
            }
        }
        assert_eq!(flushed, "abcde", "every character reached a flush");
    }

    #[test]
    fn text_gate_never_flushes_an_empty_payload() {
        let mut g = TextGate::new();
        assert_eq!(g.push("", 0), None);
        // …and the suppressed empty push must not consume the first-flush
        // allowance, or the next real fragment would be delayed a full gap.
        assert_eq!(g.push("x", 1).as_deref(), Some("x"));
    }

    #[test]
    fn text_gate_survives_clock_skew() {
        let mut g = TextGate::new();
        assert_eq!(g.push("a", 1000).as_deref(), Some("a"));
        assert_eq!(g.push("b", 999), None, "saturating subtraction, no panic");
    }

    /// Collect emitted events into a shared `Vec`, standing in for `host.rs`'s
    /// wire writer.
    fn recording_tick_emit() -> (TickEmit, Arc<Mutex<Vec<PluginEvent>>>) {
        let events: Arc<Mutex<Vec<PluginEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(move |ev| {
            sink.lock().unwrap_or_else(|e| e.into_inner()).push(ev);
        });
        (emit, events)
    }

    #[test]
    fn hop_texter_emits_exactly_one_delta_carrying_the_fragment() {
        let _g = STREAM_TEXT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("CREW_STREAM_TEXT");
        std::env::remove_var("CREW_STREAM_TEXT"); // default: streaming enabled

        let (emit, events) = recording_tick_emit();
        let on_text = hop_texter(emit, "coder".to_string());
        on_text("hello");

        match prev {
            Some(p) => std::env::set_var("CREW_STREAM_TEXT", p),
            None => std::env::remove_var("CREW_STREAM_TEXT"),
        }

        let got = events.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(got.len(), 1, "expected exactly one Delta, got {got:?}");
        match &got[0] {
            PluginEvent::Delta { agent, text } => {
                assert_eq!((agent.as_str(), text.as_str()), ("coder", "hello"));
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn hop_texter_emits_nothing_when_stream_text_env_is_zero() {
        let _g = STREAM_TEXT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var_os("CREW_STREAM_TEXT");
        std::env::set_var("CREW_STREAM_TEXT", "0");

        // hop_texter reads the env var once, at construction — set it first.
        let (emit, events) = recording_tick_emit();
        let on_text = hop_texter(emit, "coder".to_string());
        on_text("hello");

        match prev {
            Some(p) => std::env::set_var("CREW_STREAM_TEXT", p),
            None => std::env::remove_var("CREW_STREAM_TEXT"),
        }

        assert!(
            events.lock().unwrap_or_else(|e| e.into_inner()).is_empty(),
            "CREW_STREAM_TEXT=0 must suppress every Delta"
        );
    }
}
