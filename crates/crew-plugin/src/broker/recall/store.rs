//! The graph on disk: `./.crew/recall.jsonl`, appended after every turn and
//! replayed at broker start.
//!
//! Project-scoped, like `memory.md` and the session log, and for the same
//! reason: what crew learned about THIS tree is what helps in this tree. The
//! file is append-only so a crash mid-write costs the last line, never the
//! graph; it is rewritten whole only at compaction, through a temp file and a
//! rename, so the same crash during a rewrite leaves the old file intact.
use std::io::Write;
use std::path::{Path, PathBuf};

use super::codec::{self, Rec};
use super::graph::{Graph, TURNS_KEPT};

/// Lines past which the log is pruned and rewritten. Every turn writes a
/// handful, so this is a few hundred turns of history before the first
/// compaction — long enough that most projects never see one.
const COMPACT_AT: usize = 4000;

/// `CREW_PROJECT_DIR` overrides the process CWD — the seam every disk-backed
/// broker module shares (`sessionlog::base_dir`, `memory::base_dir`), because
/// lib tests share one working directory and cannot each chdir. Production
/// never sets it.
pub(crate) fn base_dir() -> PathBuf {
    std::env::var("CREW_PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub(crate) fn path_at(base: &Path) -> PathBuf {
    base.join(".crew").join("recall.jsonl")
}

/// Replay the log into a graph. A missing file is an empty graph — a first
/// run, not an error — and so is an unreadable one: memory degrades, the
/// session does not fail.
pub(crate) fn load(path: &Path) -> Graph {
    let mut g = Graph::default();
    let Ok(text) = std::fs::read_to_string(path) else {
        return g;
    };
    for line in text.lines() {
        match codec::decode(line) {
            Some(Rec::Node(n)) => g.put(n),
            Some(Rec::Edge(f, t, rel, w)) => g.put_edge(f, t, rel, w),
            None => {}
        }
    }
    g
}

/// Append records, best-effort: a memory that cannot be written must never
/// break the turn that produced it.
pub(crate) fn append(path: &Path, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    for l in lines {
        let _ = writeln!(f, "{l}");
    }
}

/// Every node then every edge, the form a fresh load rebuilds exactly.
pub(crate) fn dump(g: &Graph) -> Vec<String> {
    let mut out: Vec<String> = g.nodes().map(|(_, n)| codec::node_line(n)).collect();
    for e in g.edges() {
        let (Some(from), Some(to)) = (g.node(e.from), g.node(e.to)) else {
            continue;
        };
        out.push(codec::edge_line(e, from, to));
    }
    out
}

/// Prune and rewrite when the log has grown past [`COMPACT_AT`] lines.
/// Returns whether it ran, which is what the test asserts on.
pub(crate) fn compact_if_needed(path: &Path, g: &mut Graph) -> bool {
    let lines = std::fs::read_to_string(path)
        .map(|t| t.lines().count())
        .unwrap_or(0);
    if lines <= COMPACT_AT {
        return false;
    }
    g.prune(TURNS_KEPT);
    let tmp = path.with_extension("jsonl.tmp");
    let body = dump(g).join("\n");
    if std::fs::write(&tmp, format!("{body}\n")).is_ok() {
        let _ = std::fs::rename(&tmp, path);
        return true;
    }
    let _ = std::fs::remove_file(&tmp);
    false
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
