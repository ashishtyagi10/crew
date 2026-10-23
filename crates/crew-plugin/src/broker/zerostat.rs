//! The reply stat a hop gets when its backend reported no usage: the latency
//! alone, everything else zero, no tool pool. Both engines close every reply
//! with one — the relay's untimed CLI hops (`relay.rs`), the fan-out's failed
//! agents (`fan.rs`) — so the host's dial reconciles and the reply lifecycle
//! never stays open; written once so the two cannot drift on which zeros
//! mean "unknown". A reply that DID report usage gets [`with_usage`], for
//! the same reason.
use std::time::Duration;

use super::adapter::Usage;
use crate::protocol::PluginEvent;

/// `agent` took `elapsed` and spent `u`: its real tokens, its prompt as the
/// context fill, its cost.
pub(crate) fn with_usage(agent: &str, elapsed: Duration, u: &Usage) -> PluginEvent {
    PluginEvent::Stats {
        exchanges: 0,
        tokens: u64::from(u.input_tokens + u.output_tokens),
        agent: agent.to_string(),
        ms: elapsed.as_millis() as u64,
        ctx: u64::from(u.input_tokens),
        tok_in: u64::from(u.input_tokens),
        tok_out: u64::from(u.output_tokens),
        cost_microusd: u.cost_microusd,
        tools: None,
    }
}

/// `agent` took `elapsed` and reported nothing else.
pub(crate) fn latency_only(agent: &str, elapsed: Duration) -> PluginEvent {
    PluginEvent::Stats {
        exchanges: 0,
        tokens: 0,
        agent: agent.to_string(),
        ms: elapsed.as_millis() as u64,
        ctx: 0,
        tok_in: 0,
        tok_out: 0,
        cost_microusd: 0,
        tools: None,
    }
}
