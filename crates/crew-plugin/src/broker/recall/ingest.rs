//! Writing one turn into the graph: the turn node, the topics and files it
//! was about, the joins between them, and the log lines for exactly those.
//!
//! Only what changed is appended. The alternative — dumping the whole graph
//! after every turn — is simpler and was the first version, and it turns a
//! 40-node graph into a 40-line write per turn and a compaction every few
//! turns. What changed is a handful of lines whatever the graph weighs.
use super::codec::{edge_line, node_line};
use super::extract;
use super::node::{Edge, Kind, NodeId, Rel};
use super::{now_ms, Recall, ANSWER_KEPT, CROSS_LINKED};

/// Turns recorded by this process, so two of them inside one millisecond are
/// two nodes. They were one before this counter existed — the clock is the
/// key, and a fast pane (or a test) merged the turns it answered quickest.
static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Recall {
    /// Remember one finished exchange. Best-effort on disk; the in-memory
    /// graph is updated either way, so a read-only checkout still recalls
    /// within the session.
    pub(crate) fn record(&mut self, asked: &str, answered: &str) {
        let (asked, answered) = (asked.trim(), answered.trim());
        if !self.on || asked.is_empty() || answered.is_empty() {
            return;
        }
        let now = now_ms();
        let text = format!(
            "you asked: {asked}\ncrew answered: {}",
            crate::broker::thread::clip_chars(answered, ANSWER_KEPT)
        );
        let mut w = Writer::default();
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let turn = w.node(self, Kind::Turn, &format!("{now}-{seq}"), &text, now);
        let both = format!("{asked}\n{answered}");
        let mut topics: Vec<NodeId> = Vec::new();
        for t in extract::topics(&both) {
            let id = w.node(self, Kind::Topic, &t, &t, now);
            w.edge(self, turn, id, Rel::Mentions);
            topics.push(id);
        }
        for p in extract::paths(&both) {
            let id = w.node(self, Kind::File, &p, &p, now);
            w.edge(self, turn, id, Rel::Mentions);
        }
        for u in extract::urls(&both) {
            let id = w.node(self, Kind::Page, &u, &u, now);
            w.edge(self, turn, id, Rel::Mentions);
        }
        for (i, a) in topics.iter().take(CROSS_LINKED).enumerate() {
            for b in topics.iter().take(CROSS_LINKED).skip(i + 1) {
                w.edge(self, *a, *b, Rel::With);
            }
        }
        if let Some(prev) = self.prev.replace(turn) {
            w.edge(self, prev, turn, Rel::Then);
        }
        w.flush(self);
    }
}

/// Files a task changed, cross-linked so the graph knows they travel
/// together.
///
/// A turn's TEXT names the files the model talked about; this names the ones
/// it actually wrote. They are rarely the same list, and the difference is
/// the useful part: after a task edits `nav.rs` and `linecap-debt.txt`, a
/// later question about either one recalls the other, because in this project
/// they change together — which is exactly the thing a newcomer to a tree
/// (and a model starting a fresh session) has no way to know.
impl Recall {
    pub(crate) fn record_changes(&mut self, paths: &[String]) {
        if !self.on || paths.len() < 2 {
            return; // one file changing alone says nothing about any other
        }
        let now = now_ms();
        let mut w = Writer::default();
        let ids: Vec<NodeId> = paths
            .iter()
            .take(CHANGED_MAX)
            .map(|p| w.node(self, Kind::File, p, p, now))
            .collect();
        for (i, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(i + 1) {
                w.edge(self, *a, *b, Rel::With);
            }
        }
        if let Some(turn) = self.prev {
            for id in &ids {
                w.edge(self, turn, *id, Rel::Mentions);
            }
        }
        w.flush(self);
    }
}

/// Files cross-linked per task. Six is fifteen edges; a sweeping refactor
/// that touches forty files says nothing about any particular pair.
const CHANGED_MAX: usize = 6;

/// The lines one `record` will append, collected as it touches the graph.
#[derive(Default)]
struct Writer {
    lines: Vec<String>,
}

impl Writer {
    fn node(&mut self, r: &mut Recall, kind: Kind, key: &str, text: &str, ms: u64) -> NodeId {
        let id = r.g.touch(kind, key, text, ms);
        if let Some(n) = r.g.node(id) {
            self.lines.push(node_line(n));
        }
        id
    }

    fn edge(&mut self, r: &mut Recall, from: NodeId, to: NodeId, rel: Rel) {
        let Some(e) = r.g.link(from, to, rel) else {
            return;
        };
        self.push_edge(r, &e);
    }

    fn push_edge(&mut self, r: &Recall, e: &Edge) {
        if let (Some(from), Some(to)) = (r.g.node(e.from), r.g.node(e.to)) {
            self.lines.push(edge_line(e, from, to));
        }
    }

    fn flush(self, r: &mut Recall) {
        super::store::append(&r.path, &self.lines);
        super::store::compact_if_needed(&r.path, &mut r.g);
    }
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;
