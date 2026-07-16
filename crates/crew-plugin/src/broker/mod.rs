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
        /// Keys to restore to their prior value (or absence) on drop, beyond
        /// the always-cleared `CREW_BROKER_MOCK_REPLY`/`CREW_PROJECT_DIR`.
        restore: Vec<(&'static str, Option<String>)>,
    }

    impl Drop for MockEnv {
        fn drop(&mut self) {
            std::env::remove_var("CREW_BROKER_MOCK_REPLY");
            std::env::remove_var("CREW_PROJECT_DIR");
            if let Some(d) = &self.dir {
                let _ = std::fs::remove_dir_all(d);
            }
            for (k, v) in self.restore.drain(..) {
                match v {
                    Some(v) => std::env::set_var(k, v),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    /// A fresh, empty project dir for `CREW_PROJECT_DIR` to point at. Every
    /// mocked test gets one of these — even plain [`mock`] — so a store write
    /// (`specialists::record`/`touch`) never lands in the crate's own
    /// `./.crew/`, and no test can read another test's leftover file there.
    fn empty_project_dir() -> PathBuf {
        let id = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("crew-testenv-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".crew")).unwrap();
        dir
    }

    pub(crate) fn mock(reply: &str) -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CREW_BROKER_MOCK_REPLY", reply);
        let dir = empty_project_dir();
        std::env::set_var("CREW_PROJECT_DIR", &dir);
        MockEnv {
            guard,
            dir: Some(dir),
            restore: Vec::new(),
        }
    }

    /// Provider keys `roster_with` auto-discovers from, plus the forcing
    /// `CREW_PROVIDER` override.
    const PROVIDER_KEYS: &[&str] = &[
        "DASHSCOPE_API_KEY",
        "OPENROUTER_API_KEY",
        "ANTHROPIC_API_KEY",
        "CREW_PROVIDER",
    ];

    /// Force `roster_with`'s provider discovery to fail, deterministically —
    /// even on a machine that exports a real key (this one has
    /// `DASHSCOPE_API_KEY` in the login shell). Clears every auto-discovered
    /// key and `CREW_PROVIDER` for the guard's lifetime, restoring each to its
    /// prior value (present or absent) on drop. Also points `CREW_PROJECT_DIR`
    /// at a fresh empty dir, same as [`mock`]. For tests proving the
    /// plugin-only fallback works when no provider resolves.
    pub(crate) fn no_provider() -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let restore = PROVIDER_KEYS
            .iter()
            .map(|&k| (k, std::env::var(k).ok()))
            .collect();
        for k in PROVIDER_KEYS {
            std::env::remove_var(k);
        }
        let dir = empty_project_dir();
        std::env::set_var("CREW_PROJECT_DIR", &dir);
        MockEnv {
            guard,
            dir: Some(dir),
            restore,
        }
    }

    /// [`mock`], plus a project dir seeded with `specialists` — the roster the
    /// broker will discover. Tests that need named agents supply them here
    /// rather than relying on any inbuilt default: there isn't one any more.
    pub(crate) fn mock_with_specialists(reply: &str, specialists: &[(&str, &str)]) -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("CREW_BROKER_MOCK_REPLY", reply);
        let dir = empty_project_dir();
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
            restore: Vec::new(),
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
