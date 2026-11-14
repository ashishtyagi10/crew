//! One file, one node: a bare file name is the path the graph already knows
//! it by.
//!
//! A model says `panelcard_tests.rs` in one turn and
//! `crates/crew-app/src/panelcard_tests.rs` in the next. Filed as written
//! they were two nodes, so the file's evidence was split between them and
//! the recalled files line named it twice (live, 2026-11-14). A bare name
//! is folded into the full path only when exactly ONE known path ends with
//! it: two `mod.rs` are two files, and a guess would join them.
use super::graph::Graph;
use super::node::{Kind, NodeId};

/// The file node `p` means: its own, else the only known path it names.
pub(crate) fn id(g: &Graph, p: &str) -> Option<NodeId> {
    g.find(Kind::File, p).or_else(|| only_path_named(g, p))
}

/// How `p` is filed: the known path it names, else as written.
pub(crate) fn filed(g: &Graph, p: &str) -> String {
    match g.find(Kind::File, p) {
        Some(_) => p.to_string(),
        None => only_path_named(g, p)
            .and_then(|id| g.node(id))
            .map_or_else(|| p.to_string(), |n| n.text.clone()),
    }
}

/// `names` without the bare names a fuller path in the list already covers
/// (graphs written before the fold still hold both).
pub(crate) fn once(names: Vec<String>) -> Vec<String> {
    let covered = |bare: &String| {
        !bare.contains('/')
            && names.iter().any(|n| {
                n.to_lowercase()
                    .ends_with(&format!("/{}", bare.to_lowercase()))
            })
    };
    names.iter().filter(|n| !covered(n)).cloned().collect()
}

fn only_path_named(g: &Graph, bare: &str) -> Option<NodeId> {
    if bare.contains(['/', '\\']) {
        return None;
    }
    let tail = format!("/{}", bare.trim().to_lowercase());
    let mut hits = g
        .nodes()
        .filter(|(_, n)| n.kind == Kind::File && n.key.ends_with(&tail));
    let first = hits.next()?;
    hits.next().is_none().then_some(first.0)
}

#[cfg(test)]
#[path = "samefile_tests.rs"]
mod tests;
