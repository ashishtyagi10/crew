//! Git status for every pane's directory, kept off the winit thread.
//!
//! The sidebar has watched crew's own cwd for a while ([`crate::git`]); this
//! watches the directory each *pane* is in, which is the one you are looking
//! at when you look at that card. Panes in the same repo share one query.
//!
//! Two rules keep it cheap: at most **one** `git status` runs at a time
//! across the whole fleet, and a directory is re-asked at most every
//! [`POLL_SECS`] seconds. `git status` can take seconds on a large or
//! network-mounted repo, and running one inline froze every pane the last
//! time it was tried — so the answer always arrives through a channel.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use crate::git::GitInfo;

/// Minimum seconds between queries for one directory.
const POLL_SECS: u64 = 3;

type Answer = (PathBuf, Option<GitInfo>);

/// The directory that should be asked next: the one that is due and has been
/// waiting longest. Pure, so the schedule is testable without a repo.
fn next_due(asked: &HashMap<PathBuf, u64>, dirs: &[PathBuf], now: u64) -> Option<PathBuf> {
    dirs.iter()
        .filter(|d| {
            asked
                .get(*d)
                .is_none_or(|&t| now.saturating_sub(t) >= POLL_SECS)
        })
        .min_by_key(|d| asked.get(*d).copied().unwrap_or(0))
        .cloned()
}

#[derive(Default)]
pub(crate) struct GitFleet {
    /// Last answer per directory; `Some(None)` means "asked, not a repo".
    known: HashMap<PathBuf, Option<GitInfo>>,
    /// Unix second each directory was last asked about.
    asked: HashMap<PathBuf, u64>,
    /// The one query allowed to be in flight.
    rx: Option<Receiver<Answer>>,
}

impl GitFleet {
    /// Collect a finished answer and, if the line is free, start the stalest
    /// directory that is due. Called once a frame; does nothing at all on the
    /// frames between.
    pub(crate) fn poll(&mut self, dirs: &[PathBuf], now: u64) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok((dir, info)) => {
                    self.known.insert(dir, info);
                    self.rx = None;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => self.rx = None,
            }
        }
        // Panes close; their answers should not outlive them.
        self.known.retain(|d, _| dirs.contains(d));
        self.asked.retain(|d, _| dirs.contains(d));
        let Some(dir) = next_due(&self.asked, dirs, now) else {
            return;
        };
        self.asked.insert(dir.clone(), now);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let info = crate::git::query(&dir);
            let _ = tx.send((dir, info));
        });
        self.rx = Some(rx);
    }

    /// What is known about `dir` right now — `None` until the first answer
    /// arrives, and for a directory that is not a repo.
    pub(crate) fn info(&self, dir: Option<&Path>) -> Option<&GitInfo> {
        self.known.get(dir?)?.as_ref()
    }
}

#[cfg(test)]
#[path = "gitfleet_tests.rs"]
mod tests;
