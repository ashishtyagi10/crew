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
pub mod approval;
mod ask;
pub(crate) mod auth;
mod capabilities;
mod changed;
mod checkpoint;
mod commands;
mod compact;
mod constructs;
mod diff;
mod directs;
mod discover;
mod doctor;
mod doctorprobe;
mod elect;
mod engine;
mod fan;
mod gitmsg;
mod hop;
pub(crate) mod integration;
mod intent;
pub mod ledger;
mod logincmd;
mod memory;
mod modelcmd;
mod modelpick;
mod normalize;
mod plan;
mod plugins;
mod registry;
mod relay;
mod retired;
mod review;
mod roundloop;
mod route;
mod run;
mod session;
mod sessionlog;
mod shellenv;
mod signin;
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
pub mod tier;
mod toolcall;
mod toolclip;
mod toolline;
mod toolpick;

pub use adapter::{Adapter, CliAdapter, Normalize};
pub use agents::known_adapters;
pub use ask::{explain_output, suggest_command, suggest_far_command};
pub use commands::{broker_constructs, construct_summary, expand_alias};
pub use discover::{
    direct_by_name, no_provider_advice, pick_provider as active_provider, DirectProvider,
    ProviderKind as Provider, DIRECT,
};
pub use engine::Broker;
pub use hop::{Hop, HopKind, RunStats};
pub use skills::{list as skills_list, Skill};
pub use {registry::Registry, route::parse_routing, route::Routing};
pub use {stdio::run_broker_stdio, toolcall::ToolRunner};

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
    /// fixture. Election is the model's call (`broker::elect`) with a
    /// roster-first fallback under the mock, so the ORDER is load-bearing.
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

    impl MockEnv {
        /// Whether this guard captured `k` and will therefore put its
        /// pre-guard value back on drop. Lets a test assert the restore
        /// contract without reading process env *after* the lock is released,
        /// which would race every other test in the binary.
        pub(crate) fn restores(&self, k: &str) -> bool {
            self.restore.iter().any(|(rk, _)| *rk == k)
        }
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
    /// `CREW_PROVIDER` override and the credential store's own path override
    /// (`credentials::path`) — `forced_provider()` and `shellenv::hydrate()`
    /// both read the store, so a real `credentials.json` is exactly as much a
    /// source of a provider pin as the four env vars are.
    fn provider_keys() -> Vec<&'static str> {
        // From `credentials::VARS`, not a list: providers are a table now, and
        // a guard that promises "no provider will resolve" while clearing only
        // three of six keys does not fail loudly — it quietly stops guarding
        // on any machine that exports one of the others.
        let mut v: Vec<&'static str> = crate::credentials::VARS.to_vec();
        v.push("CREW_PROVIDER");
        v.push("CREW_CREDENTIALS_PATH");
        v
    }

    /// Force `roster_with`'s provider discovery to fail, deterministically —
    /// even on a machine that exports a real key (this one has
    /// `DASHSCOPE_API_KEY` in the login shell) or has saved a real credential
    /// pin through the in-app key popup (`credentials::save_key`, backed by
    /// `~/.config/crew/credentials.json` or equivalent). Clears every
    /// auto-discovered key and `CREW_PROVIDER` for the guard's lifetime, and
    /// points `CREW_CREDENTIALS_PATH` at a sibling path that cannot exist, so
    /// `credentials::load()` (reached via `forced_provider()` and
    /// `shellenv::hydrate()`) reads as empty rather than the real store.
    /// Restores every variable to its prior value (present or absent) on
    /// drop. Also points `CREW_PROJECT_DIR` at a fresh empty dir, same as
    /// [`mock`]. For tests proving the plugin-only fallback works when no
    /// provider resolves.
    pub(crate) fn no_provider() -> MockEnv {
        no_provider_inner(None, None)
    }

    /// [`no_provider`] on a machine that already has `CREW_CREDENTIALS_PATH`
    /// pointing at a real store: `masked` is set *inside the guard's lock*,
    /// before the neutralisation runs, so the guard has to capture and
    /// override a live value rather than an absent one. A test cannot do this
    /// itself — setting the variable before calling `no_provider()` would
    /// mutate process-global env outside the lock, racing every other test in
    /// this binary. The pre-guard value is still what's restored on drop, so
    /// `masked` never escapes the guard's lifetime.
    pub(crate) fn no_provider_masking(masked: &std::path::Path) -> MockEnv {
        no_provider_inner(Some(masked), None)
    }

    /// No provider in the ENVIRONMENT, but the credential store at `store`
    /// left visible — the machine where the in-app key popup saved the only
    /// key there is. Everything else matches [`no_provider`], including the
    /// lock and the restore-on-drop.
    pub(crate) fn no_provider_with_store(store: &std::path::Path) -> MockEnv {
        no_provider_inner(None, Some(store))
    }

    /// The shared body of the `no_provider*` guards. `masked` is a store path
    /// to install *before* neutralising (so the neutralisation has something
    /// to override); `store` is the store path to leave in place *after* it
    /// (defaulting to a path inside the throwaway project dir, which cannot
    /// exist).
    fn no_provider_inner(
        masked: Option<&std::path::Path>,
        store: Option<&std::path::Path>,
    ) -> MockEnv {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let restore = provider_keys()
            .iter()
            .map(|&k| (k, std::env::var(k).ok()))
            .collect();
        if let Some(m) = masked {
            std::env::set_var("CREW_CREDENTIALS_PATH", m);
        }
        for k in provider_keys() {
            std::env::remove_var(k);
        }
        let dir = empty_project_dir();
        std::env::set_var("CREW_PROJECT_DIR", &dir);
        let cred = store.map_or_else(|| dir.join("credentials.json"), PathBuf::from);
        std::env::set_var("CREW_CREDENTIALS_PATH", cred);
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
#[path = "mod_tests.rs"]
mod tests;
