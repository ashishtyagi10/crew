//! The in-memory graph: nodes by `(kind, key)`, edges as a flat list.
//!
//! Flat, not indexed by node: the working set is a few thousand edges and the
//! only read is a two-hop spread from a handful of seeds, so a scan costs
//! microseconds and an adjacency map would cost a second invariant to keep in
//! step with the log. When a scan stops being cheap the fix is a smaller
//! graph (see [`Graph::prune`]), not a bigger index.
use std::collections::HashMap;

use super::node::{Edge, Kind, Node, NodeId, Rel};

/// Turns kept on disk. Past this the oldest are pruned at compaction, with
/// the topics they were the last mention of.
pub(crate) const TURNS_KEPT: usize = 400;

#[derive(Debug, Default)]
pub(crate) struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    index: HashMap<(Kind, String), NodeId>,
}

impl Graph {
    /// Insert or bump `(kind, key)`, returning its id. A repeat sighting adds
    /// a hit and moves the stamp forward; `text` is only written on insert,
    /// so the first spelling of a file path is the one shown.
    pub(crate) fn touch(&mut self, kind: Kind, key: &str, text: &str, ms: u64) -> NodeId {
        let key = key.trim().to_lowercase();
        if let Some(&id) = self.index.get(&(kind, key.clone())) {
            let n = &mut self.nodes[id as usize];
            n.hits = n.hits.saturating_add(1);
            n.last_ms = n.last_ms.max(ms);
            return id;
        }
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node {
            kind,
            key: key.clone(),
            text: text.trim().to_owned(),
            hits: 1,
            last_ms: ms,
        });
        self.index.insert((kind, key), id);
        id
    }

    /// Insert or strengthen `from -rel-> to`, returning the edge as it now
    /// stands so the caller can persist exactly what changed. Self-links are
    /// dropped (`None`): a topic co-occurring with itself is a loop that only
    /// ever inflates ranking.
    pub(crate) fn link(&mut self, from: NodeId, to: NodeId, rel: Rel) -> Option<Edge> {
        if from == to {
            return None;
        }
        if let Some(e) = self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.rel == rel)
        {
            e.weight = e.weight.saturating_add(1);
            return Some(*e);
        }
        let e = Edge {
            from,
            to,
            rel,
            weight: 1,
        };
        self.edges.push(e);
        Some(e)
    }

    /// Restore a node exactly as the log recorded it (load path only).
    pub(crate) fn put(&mut self, n: Node) {
        let k = (n.kind, n.key.clone());
        match self.index.get(&k) {
            Some(&id) => self.nodes[id as usize] = n,
            None => {
                self.index.insert(k, self.nodes.len() as NodeId);
                self.nodes.push(n);
            }
        }
    }

    /// Restore an edge (load path only); a dangling endpoint drops the edge.
    pub(crate) fn put_edge(&mut self, from: (Kind, String), to: (Kind, String), rel: Rel, w: u32) {
        let (Some(&from), Some(&to)) = (self.index.get(&from), self.index.get(&to)) else {
            return;
        };
        if from == to {
            return;
        }
        match self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.rel == rel)
        {
            Some(e) => e.weight = w,
            None => self.edges.push(Edge {
                from,
                to,
                rel,
                weight: w,
            }),
        }
    }

    pub(crate) fn find(&self, kind: Kind, key: &str) -> Option<NodeId> {
        self.index.get(&(kind, key.trim().to_lowercase())).copied()
    }

    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id as usize)
    }

    pub(crate) fn nodes(&self) -> impl Iterator<Item = (NodeId, &Node)> {
        self.nodes.iter().enumerate().map(|(i, n)| (i as NodeId, n))
    }

    pub(crate) fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Both directions: memory does not care which end of an edge you enter
    /// from — a turn leads to its topics and a topic back to its turns.
    pub(crate) fn neighbors(&self, id: NodeId) -> impl Iterator<Item = (NodeId, u32)> + '_ {
        self.edges.iter().filter_map(move |e| {
            if e.from == id {
                Some((e.to, e.weight))
            } else if e.to == id {
                Some((e.from, e.weight))
            } else {
                None
            }
        })
    }

    /// Node count — what the tests measure a load or a prune by; the app
    /// asks [`Self::count`] per kind (`/doctor`) and never the total.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn count(&self, kind: Kind) -> usize {
        self.nodes.iter().filter(|n| n.kind == kind).count()
    }

    /// Drop the oldest turns past `keep`, and any node left with no edges.
    /// Rebuilds ids, so it may only run where the whole file is rewritten.
    pub(crate) fn prune(&mut self, keep: usize) {
        let mut turns: Vec<(NodeId, u64)> = self
            .nodes()
            .filter(|(_, n)| n.kind == Kind::Turn)
            .map(|(id, n)| (id, n.last_ms))
            .collect();
        turns.sort_unstable_by_key(|(_, ms)| std::cmp::Reverse(*ms));
        let doomed: Vec<NodeId> = turns.into_iter().skip(keep).map(|(id, _)| id).collect();
        if doomed.is_empty() {
            return;
        }
        self.edges
            .retain(|e| !doomed.contains(&e.from) && !doomed.contains(&e.to));
        let linked: Vec<NodeId> = self
            .nodes()
            .filter(|(id, _)| self.edges.iter().any(|e| e.from == *id || e.to == *id))
            .map(|(id, _)| id)
            .collect();
        let kept: Vec<Node> = linked
            .iter()
            .filter_map(|id| self.node(*id).cloned())
            .collect();
        let remap: HashMap<NodeId, NodeId> = linked
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new as NodeId))
            .collect();
        self.edges
            .retain(|e| remap.contains_key(&e.from) && remap.contains_key(&e.to));
        for e in &mut self.edges {
            e.from = remap[&e.from];
            e.to = remap[&e.to];
        }
        self.nodes = kept;
        self.index = self
            .nodes()
            .map(|(id, n)| ((n.kind, n.key.clone()), id))
            .collect();
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
