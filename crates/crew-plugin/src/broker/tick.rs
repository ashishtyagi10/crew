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

#[cfg(test)]
mod tests {
    use super::*;

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
}
