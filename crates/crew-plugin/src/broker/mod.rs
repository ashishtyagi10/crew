//! Multi-agent broker: routes messages between headless CLI coding agents
//! (claude, codex, opencode) discovered at runtime. The broker is
//! agent-agnostic — adding an agent means adding one adapter in `agents` and
//! registering it in `known_adapters`; nothing in the routing engine changes.
//!
//! Every message in flight is an [`Envelope`]. An adapter turns an envelope
//! body into a clean reply string (never raw CLI chatter). The [`engine::Broker`]
//! drives the relay: it calls the addressed agent, parses the reply for a
//! routing directive (`TO <peer>:` / `DONE`), logs every hop, and stops at the
//! hop limit so a thread can never loop forever.
mod adapter;
mod agents;
mod engine;
mod hop;
mod normalize;
mod registry;
mod route;
mod run;
mod stdio;

pub use adapter::{Adapter, CliAdapter, Normalize};
pub use agents::known_adapters;
pub use engine::Broker;
pub use hop::{Hop, HopKind, RunStats};
pub use registry::Registry;
pub use route::{parse_routing, Routing};
pub use stdio::run_broker_stdio;

/// A single message addressed from one agent to another. Every message and
/// reply that flows through the broker takes this shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub from: String,
    pub to: String,
    pub thread_id: String,
    /// How many relays deep this message is; the broker caps it (loop guard).
    pub hop: u32,
    pub body: String,
}

impl Envelope {
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        thread_id: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            thread_id: thread_id.into(),
            hop: 0,
            body: body.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_new_starts_at_hop_zero() {
        let e = Envelope::new("user", "claude", "t1", "hi");
        assert_eq!(
            (e.from.as_str(), e.to.as_str(), e.hop),
            ("user", "claude", 0)
        );
        assert_eq!(e.thread_id, "t1");
        assert_eq!(e.body, "hi");
    }
}
