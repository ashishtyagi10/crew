//! What the user types into a swarm while it runs, for every worker in it.
//!
//! The relay's steer (crew-plugin `broker::steer`) joins ONE loop: the first
//! round to come round takes what is waiting. A swarm runs several workers at
//! once, and handing "also check the tests" to whichever reached a round first
//! gives it to one arbitrary worker and hides it from the rest — including
//! the ones not started yet, which are the ones best placed to act on it. So
//! a run keeps one log: what is taken goes in it, and every worker reads the
//! whole log at each round, a worker that starts late in its first prompt.
//!
//! crew-hive cannot reach the host's inbox (the dependency points the other
//! way), so the host hands in `source`: what arrived since it was last asked,
//! announced to the pane as it is taken. It is only called from a worker's
//! own future, between rounds.

use std::sync::{Arc, Mutex};

/// The heading of the section a steer rides in, for the relay and the swarm
/// alike.
pub const HEAD: &str = "THE USER ADDED WHILE YOU WORKED:";

/// The line under it in a swarm worker's prompt. The worker has one part of
/// the request; what the user said was to the whole crew.
pub const WORKER_TAIL: &str = "This was said to the whole crew: act on it where it \
                             bears on your task, and where it differs from your task \
                             above, it replaces it.";

type Source = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

/// One run's steers: a source to take new ones from, and the log of every
/// one taken so far, shared by all its workers.
#[derive(Clone)]
pub struct Steers {
    source: Source,
    log: Arc<Mutex<Vec<String>>>,
}

impl Steers {
    pub fn new(source: impl Fn() -> Vec<String> + Send + Sync + 'static) -> Self {
        Self {
            source: Arc::new(source),
            log: Arc::default(),
        }
    }

    /// Everything said to this run so far, taking what is new first.
    pub fn all(&self) -> Vec<String> {
        let new = (self.source)();
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.extend(new.into_iter().filter(|t| !t.trim().is_empty()));
        log.clone()
    }

    /// What workers took, without taking more: for the closing answer, which
    /// should honour what the crew was told, not something typed after the
    /// last worker finished (that one the pane sends as the next turn).
    pub fn taken(&self) -> Vec<String> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// The section for `added` (empty when there is none): [`HEAD`], one dash per
/// message, a message of several lines kept one item with its later lines
/// indented under the dash, then `tail`, then a blank line.
pub fn section(added: &[String], tail: &str) -> String {
    if added.is_empty() {
        return String::new();
    }
    let items: Vec<String> = added
        .iter()
        .map(|t| format!("- {}", t.trim().replace('\n', "\n  ")))
        .collect();
    format!("{HEAD}\n{}\n{tail}\n\n", items.join("\n"))
}

#[cfg(test)]
#[path = "steers_tests.rs"]
mod tests;
