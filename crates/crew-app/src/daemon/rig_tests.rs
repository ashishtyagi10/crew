//! The daemon under test: a spawner that remembers who each session was opened for and what was
//! written into it, a channel that goes nowhere, and a watchlist of the test's own — never the
//! user's, which a firing would otherwise consume for real.
use crate::channel::loopback::Loopback;
use crate::daemon::intentlog::Watchlist;
use crate::daemon::session::{SessionProc, Spawner};
use crate::daemon::Daemon;
use std::path::Path;

/// A session process that exists and remembers what was written to it; the registry's own
/// behaviour is covered in `session_tests`.
struct Idle(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
impl SessionProc for Idle {
    fn alive(&mut self) -> bool {
        true
    }
    fn kill(&mut self) {}
    fn send(&mut self, line: &str) -> bool {
        self.0.lock().unwrap().push(line.to_string());
        true
    }
    fn output(&self) -> (Vec<String>, usize) {
        (Vec::new(), 0)
    }
}

/// A spawner that remembers what requester each session was opened for, which is the whole
/// security question for a scheduled run.
struct Recorder {
    opened: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    written: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}
impl Spawner for Recorder {
    fn spawn(
        &mut self,
        _cwd: Option<&Path>,
        requester: Option<&str>,
    ) -> std::io::Result<Box<dyn SessionProc>> {
        self.opened
            .lock()
            .unwrap()
            .push(requester.unwrap_or("<none>").to_string());
        Ok(Box::new(Idle(std::sync::Arc::clone(&self.written))))
    }
}

pub(crate) struct Rig {
    pub d: Daemon,
    pub wire: std::sync::Arc<std::sync::Mutex<crate::channel::loopback::Wire>>,
    pub opened: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Every line written into a session — what the agent was actually told.
    pub written: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    pub watch: Watchlist,
}

impl Rig {
    /// The TEXT of every task handed to an agent, dug out of the broker commands the bridge
    /// writes. What a channel carried is the only thing a parity test can compare.
    pub(super) fn tasks(&self) -> Vec<String> {
        self.written
            .lock()
            .unwrap()
            .iter()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v.get("type").and_then(|o| o.as_str()) == Some("send"))
            .filter_map(|v| v.get("text")?.as_str().map(str::to_string))
            .collect()
    }
}

/// A daemon with a channel that goes nowhere and a watchlist of this test's own — never the
/// user's, which a firing would otherwise consume for real.
pub(crate) fn rig(tag: &str) -> Rig {
    let opened = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut d = Daemon::with_spawner(Box::new(Recorder {
        opened: std::sync::Arc::clone(&opened),
        written: std::sync::Arc::clone(&written),
    }));
    let (c, wire) = Loopback::pair("test");
    d.add_channel(Box::new(c));
    let path = std::env::temp_dir().join(format!("crew-clock-{}-{tag}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    d.set_watchlist(Watchlist::at(&path));
    Rig {
        d,
        wire,
        opened,
        written,
        watch: Watchlist::at(&path),
    }
}

pub(crate) fn sent(r: &Rig) -> Vec<(String, String)> {
    r.wire.lock().unwrap().outbox.clone()
}
