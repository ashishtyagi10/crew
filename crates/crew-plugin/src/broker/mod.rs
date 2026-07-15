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
mod ask;
mod checkpoint;
mod commands;
mod constructs;
mod diff;
mod discover;
mod doctor;
mod engine;
mod fan;
mod gitmsg;
mod hop;
mod memory;
mod normalize;
mod plan;
mod plugins;
mod registry;
mod relay;
mod review;
mod route;
mod run;
mod session;
mod sessionlog;
mod shellenv;
mod skillframe;
mod skills;
pub(crate) mod specialists;
mod standup;
mod stdio;
mod swarm;
mod sysread;
mod sysrun;
mod systools;
mod tasks;
mod tick;
mod toolcall;
mod toolclip;

pub use adapter::{Adapter, CliAdapter, Normalize};
pub use agents::known_adapters;
pub use ask::{explain_output, suggest_command, suggest_far_command};
pub use engine::Broker;
pub use hop::{Hop, HopKind, RunStats};
pub use registry::Registry;
pub use route::{parse_routing, Routing};
pub use stdio::run_broker_stdio;
pub use toolcall::ToolRunner;

/// Serialises tests that set `CREW_BROKER_MOCK_REPLY` / `CREW_PROJECT_DIR`
/// (process-wide env): each guard holds the same global lock and removes the
/// variables again on drop.
#[cfg(test)]
pub(crate) mod testenv {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// One lock for every env-touching guard here: `mock` and
    /// `mock_with_specialists` both mutate process-wide state, so they must
    /// serialise against each other, not just against themselves.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static SEQ: AtomicU32 = AtomicU32::new(0);

    /// The roster the inbuilt trio used to hard-code, now an explicit test
    /// fixture. The roles are load-bearing, not decoration:
    /// `constructs::is_critic` elects the judge by capability words, so
    /// `reviewer` must carry "critique" or the `/goal` tests fail for a
    /// second, subtler reason.
    pub(crate) const TRIO: &[(&str, &str)] = &[
        ("planner", "planning, analysis, architecture, research"),
        ("coder", "building, implementation, synthesis"),
        ("reviewer", "review, critique, second opinion"),
    ];

    pub(crate) struct MockEnv {
        #[allow(dead_code)]
        guard: std::sync::MutexGuard<'static, ()>,
        dir: Option<PathBuf>,
    }

    impl Drop for MockEnv {
        fn drop(&mut self) {
            std::env::remove_var("CREW_BROKER_MOCK_REPLY");
            std::env::remove_var("CREW_PROJECT_DIR");
            if let Some(d) = &self.dir {
                let _ = std::fs::remove_dir_all(d);
            }
        }
    }

    pub(crate) fn mock(reply: &str) -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CREW_BROKER_MOCK_REPLY", reply);
        MockEnv { guard, dir: None }
    }

    /// [`mock`], plus a project dir seeded with `specialists` — the roster the
    /// broker will discover. Tests that need named agents supply them here
    /// rather than relying on any inbuilt default: there isn't one any more.
    pub(crate) fn mock_with_specialists(reply: &str, specialists: &[(&str, &str)]) -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CREW_BROKER_MOCK_REPLY", reply);
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("crew-testenv-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".crew")).unwrap();
        // Newest-first, matching what `specialists::save_at` writes, so the
        // seeded order is the order the roster comes back in.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let json: Vec<serde_json::Value> = specialists
            .iter()
            .map(|(name, role)| serde_json::json!({ "name": name, "role": role, "last_used": now }))
            .collect();
        std::fs::write(
            dir.join(".crew").join("specialists.json"),
            serde_json::to_string_pretty(&json).unwrap(),
        )
        .unwrap();
        std::env::set_var("CREW_PROJECT_DIR", &dir);
        MockEnv {
            guard,
            dir: Some(dir),
        }
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
