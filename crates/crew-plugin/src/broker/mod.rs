//! Multi-agent broker: routes messages between coding agents. By default these
//! are the inbuilt API agents (planner/coder/reviewer in `apiadapter`), which
//! call the LLM in-process via crew-hive; the external-CLI adapters in `agents`
//! remain available as the same [`Adapter`] abstraction. The broker is
//! agent-agnostic — an adapter turns an envelope body into a clean reply string;
//! nothing in the routing engine cares how that reply was produced.
//!
//! Every message in flight is an [`Envelope`]. An adapter turns an envelope
//! body into a clean reply string (never raw CLI chatter). The [`engine::Broker`]
//! drives the relay: it calls the addressed agent, parses the reply for a
//! routing directive (`TO <peer>:` / `DONE`), logs every hop, and stops at the
//! hop limit so a thread can never loop forever.
mod adapter;
mod agents;
mod apiadapter;
mod commands;
mod constructs;
mod discover;
mod engine;
mod fan;
mod hop;
mod normalize;
mod plan;
mod plugins;
mod registry;
mod relay;
mod route;
mod run;
mod session;
mod shellenv;
mod skills;
mod stdio;
mod toolcall;

pub use adapter::{Adapter, CliAdapter, Normalize};
pub use agents::known_adapters;
pub use engine::Broker;
pub use hop::{Hop, HopKind, RunStats};
pub use registry::Registry;
pub use route::{parse_routing, Routing};
pub use stdio::run_broker_stdio;
pub use toolcall::ToolRunner;

/// Serialises tests that set `CREW_BROKER_MOCK_REPLY` (process-wide env): the
/// guard holds a global lock and removes the variable again on drop.
#[cfg(test)]
pub(crate) mod testenv {
    pub(crate) struct MockEnv(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

    impl Drop for MockEnv {
        fn drop(&mut self) {
            std::env::remove_var("CREW_BROKER_MOCK_REPLY");
        }
    }

    pub(crate) fn mock(reply: &str) -> MockEnv {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CREW_BROKER_MOCK_REPLY", reply);
        MockEnv(g)
    }
}

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
