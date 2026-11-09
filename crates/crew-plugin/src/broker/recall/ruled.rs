//! What was ruled out: the user's corrections and a compaction's `ruled out:`
//! lines, kept so the next task on the same files is told before it retries.
//!
//! WHY in the graph and not a list of its own: a correction is only worth
//! repeating in front of work it bears on, and "bears on" is exactly what the
//! graph's walk already answers (files, then the words around them). A second
//! store would need its own relevance and its own forgetting; here a
//! correction is one more node the walk reaches, pruned with the turns.
//!
//! It is stored as a `Turn` whose text opens with [`RULED`], not a new kind:
//! the log format, the pruning and a word's turn count all keep working, and
//! the block ([`super::block`]) reads the prefix to label it as a correction
//! rather than as something crew answered.
use super::extract;
use super::ingest::Writer;
use super::node::{Kind, NodeId, Rel};
use super::{now_ms, Recall};

/// How a stored correction opens — what the block and the queries read.
pub(crate) const RULED: &str = "ruled out: ";
/// Chars of one correction kept. It is a reminder, not the conversation.
const RULED_KEPT: usize = 360;

/// Whether a stored turn's text is a correction.
pub(crate) fn is_ruled(text: &str) -> bool {
    text.starts_with(RULED)
}

/// Chars of a message compared when asking whether a correction quotes it:
/// enough to be that message, few enough to survive the clip it was kept
/// under (`correction::line` keeps 160 of the user's words).
const QUOTE_CHARS: usize = 80;

/// Whether the stored correction quotes `ask`. A correction is noted as its
/// message is routed, BEFORE that message's own recall runs, and it must not
/// come back in front of the very message that said it as something said
/// "before".
pub(crate) fn quotes(stored: &str, ask: &str) -> bool {
    let head: String = ask.trim().chars().take(QUOTE_CHARS).collect();
    !head.is_empty() && is_ruled(stored) && stored.contains(&head)
}

impl Recall {
    /// Remember that `what` was ruled out. `tie_last` joins it to the files
    /// of the turn recorded last — the turn a user's "no, not that" is
    /// about, which often names no file itself. A compaction's line is not
    /// tied: the last recorded turn is an earlier task, not the one being
    /// compacted, and its own text names the files it is about.
    ///
    /// The same words twice are one node with two hits (the key is the
    /// text), so a summary that carries a line forward through every
    /// compaction does not fill the graph with copies of it.
    pub(crate) fn record_ruled(&mut self, what: &str, tie_last: bool) {
        let what = what.trim();
        if !self.on || what.is_empty() {
            return;
        }
        let now = now_ms();
        let text = format!(
            "{RULED}{}",
            crate::broker::thread::clip_chars(what, RULED_KEPT)
        );
        let mut w = Writer::default();
        let node = w.node(self, Kind::Turn, &text, &text, now);
        for t in extract::topics(what) {
            let id = w.node(self, Kind::Topic, &t, &t, now);
            w.edge(self, node, id, Rel::Mentions);
        }
        for p in extract::paths(what) {
            let id = w.node(self, Kind::File, &p, &p, now);
            w.edge(self, node, id, Rel::Mentions);
        }
        let last_files: Vec<NodeId> = match (tie_last, self.prev) {
            (true, Some(prev)) => self
                .g
                .neighbors(prev)
                .filter(|(id, _)| self.g.node(*id).is_some_and(|n| n.kind == Kind::File))
                .map(|(id, _)| id)
                .collect(),
            _ => Vec::new(),
        };
        for id in last_files {
            w.edge(self, node, id, Rel::Mentions);
        }
        w.flush(self);
    }
}

#[cfg(test)]
#[path = "ruled_tests.rs"]
mod tests;
