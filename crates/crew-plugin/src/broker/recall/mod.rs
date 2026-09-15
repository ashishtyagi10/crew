//! The recall graph: crew's memory across sessions.
//!
//! WHY a graph and not a longer log. The thread (`thread.rs`) remembers the
//! last six turns of THIS pane and is cleared by a restart; the session log
//! remembers everything in order and is only folded in when `/resume` asks.
//! Neither can answer the question that actually comes up — "what do I
//! already know about *this*?" — because both are ordered by time and the
//! answer is ordered by relevance. So every turn is also written here as a
//! node joined to the topics and files it was about, and the next request
//! walks those joins ([`query`]) to pull back the two or three turns that
//! bear on it, from any session, however long ago.
//!
//! It is an embedded store, not a database server: an append-only JSONL log
//! replayed into memory at start (`store`), a few thousand nodes, no new
//! dependency and no daemon. `CREW_RECALL=0` turns the whole thing off.
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

mod block;
mod codec;
mod extract;
mod graph;
mod ingest;
mod node;
mod query;
mod store;

pub(crate) use block::{Recalled, RECALL_CAP};

use graph::Graph;
use node::{Kind, NodeId};

/// Chars of one answer kept in the graph. Much shorter than the thread's
/// 3 KB: this is a reminder of what was concluded, and it is on disk forever.
const ANSWER_KEPT: usize = 400;
/// Topics cross-linked to each other per turn. All eight would be 28 edges
/// for one turn; the top few are the ones that mean anything.
const CROSS_LINKED: usize = 4;

/// The slot a session owns, shared with its worker-thread snapshots — a turn
/// is recorded ON the worker that answered it, exactly like the thread.
pub(crate) type SharedRecall = Arc<Mutex<Recall>>;

/// The graph plus where it came from.
pub(crate) struct Recall {
    g: Graph,
    path: std::path::PathBuf,
    /// The turn recorded last, so the spine (`then`) can be drawn.
    prev: Option<NodeId>,
    /// `CREW_RECALL=0`, read once at open.
    on: bool,
}

impl Recall {
    /// Replay the project's log. Called once per broker start.
    pub(crate) fn open() -> Self {
        Self::open_at(&store::base_dir())
    }

    pub(crate) fn open_at(base: &Path) -> Self {
        let on = enabled(std::env::var("CREW_RECALL").ok().as_deref());
        let path = store::path_at(base);
        let g = if on {
            store::load(&path)
        } else {
            Graph::default()
        };
        Self {
            g,
            path,
            prev: None,
            on,
        }
    }

    /// What `task` recalls — the block and its counts — or `None` when the
    /// graph has nothing to say about it.
    pub(crate) fn recalled(&self, task: &str, skip: &[String]) -> Option<Recalled> {
        if !self.on {
            return None;
        }
        block::block(&self.g, task, skip, now_ms(), RECALL_CAP)
    }

    /// The block alone, for the arms that only put it in front of a task.
    pub(crate) fn context(&self, task: &str, skip: &[String]) -> Option<String> {
        self.recalled(task, skip).map(|r| r.text)
    }

    /// `(turns, topics, files)` — `/doctor`'s counts.
    pub(crate) fn stats(&self) -> (usize, usize, usize) {
        (
            self.g.count(Kind::Turn),
            self.g.count(Kind::Topic),
            self.g.count(Kind::File),
        )
    }

    #[cfg(test)]
    pub(crate) fn nodes(&self) -> usize {
        self.g.len()
    }

    /// The off graph, as `CREW_RECALL=0` builds it — a seam, so the "writes
    /// nothing" half of the switch can be asserted without a test mutating
    /// the process environment out from under the rest of the binary.
    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            on: false,
            ..Self::default()
        }
    }
}

/// An off-disk graph for tests and for a broker that cannot write.
impl Default for Recall {
    fn default() -> Self {
        Self {
            g: Graph::default(),
            path: std::path::PathBuf::new(),
            prev: None,
            on: true,
        }
    }
}

/// The `CREW_RECALL` switch: absent is on, `0` is off, anything else is on.
/// Off means no read, no write and no block — not an empty graph that still
/// opens the file.
fn enabled(v: Option<&str>) -> bool {
    v != Some("0")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

/// How long ago `then_ms` was, against the clock now — the pane's phrasing
/// for a recall's reach, shared with the block so one line cannot say `3d`
/// where the other says `2d`.
pub(crate) fn ago_now(then_ms: u64) -> String {
    block::ago(then_ms, now_ms())
}

/// The shared slot, poison-tolerant (mirrors `thread::lock`).
pub(crate) fn lock(r: &SharedRecall) -> MutexGuard<'_, Recall> {
    r.lock().unwrap_or_else(|e| e.into_inner())
}

/// Both memories in front of one task: what earlier sessions learned about
/// it (this graph), then what THIS pane has said since (`thread`), then the
/// task. Oldest to newest, the order the thread's own block already uses, so
/// the model reads one paragraph of context and not two competing ones.
///
/// The thread's requests are passed as the skip list: a turn that is already
/// quoted above must not be quoted again from the graph — the budget is
/// better spent on the session the user has forgotten.
pub(crate) fn ahead(session: &crate::broker::session::Session, task: &str) -> String {
    let skip = crate::broker::thread::lock(&session.thread).asks();
    let inner = crate::broker::thread::with_context(&session.thread, task);
    match lock(&session.recall).context(task, &skip) {
        Some(b) => format!("{b}\n\n{inner}"),
        None => inner,
    }
}

/// The files a task actually changed, cross-linked (see `ingest`).
pub(crate) fn record_changes(r: &SharedRecall, paths: &[String]) {
    lock(r).record_changes(paths);
}

/// The arms' one recording call: `None` (failed or cancelled) records nothing.
pub(crate) fn record(r: &SharedRecall, asked: &str, answered: Option<&str>) {
    if let Some(a) = answered {
        lock(r).record(asked, a);
    }
}

/// `/doctor`'s line: the mark and the detail.
pub(crate) fn doctor_line(turns: usize, topics: usize, files: usize) -> (char, String) {
    let mark = if turns > 0 { '\u{2713}' } else { '\u{2013}' };
    (
        mark,
        format!("{turns} turn(s), {topics} topic(s), {files} file(s) in the recall graph"),
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
