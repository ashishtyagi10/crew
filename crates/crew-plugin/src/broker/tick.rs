//! Tick pacing for mid-reply StatsTick emission: a pure rate gate (clock is
//! a parameter, mirroring `Tasks::attach(.., now)`) — at most one tick per
//! agent per TICK_GAP_MS, enforced by the caller holding `last_ms`.

/// Minimum gap between two StatsTicks for one agent.
pub(crate) const TICK_GAP_MS: u64 = 150;

/// First tick always passes; later ticks pass once `min_gap_ms` elapsed.
pub(crate) fn should_tick(last_ms: Option<u64>, now_ms: u64, min_gap_ms: u64) -> bool {
    last_ms.is_none_or(|l| now_ms.saturating_sub(l) >= min_gap_ms)
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
            should_tick(Some(1000), 999, TICK_GAP_MS) == false,
            "clock skew saturates, no panic"
        );
    }
}
