//! Forgetting: the oldest turns past what is kept, and every node they leave
//! with no edges.
//!
//! Split from `graph.rs` because it is the one operation that renumbers the
//! graph — ids, the key index and the per-node edge lists all change at once
//! — and so the one that must end in [`Graph::reindex`]. A child module, so
//! it reaches the same private fields the rest of the graph does.
use std::collections::{HashMap, HashSet};

use super::super::node::{Kind, Node, NodeId};
use super::Graph;

impl Graph {
    /// Drop the oldest turns past `keep`, and any node left with no edges.
    /// Rebuilds ids, so it may only run where the whole file is rewritten.
    pub(crate) fn prune(&mut self, keep: usize) {
        let mut turns: Vec<(NodeId, u64)> = self
            .nodes()
            .filter(|(_, n)| n.kind == Kind::Turn)
            .map(|(id, n)| (id, n.last_ms))
            .collect();
        turns.sort_unstable_by_key(|(_, ms)| std::cmp::Reverse(*ms));
        let doomed: HashSet<NodeId> = turns.into_iter().skip(keep).map(|(id, _)| id).collect();
        if doomed.is_empty() {
            return;
        }
        self.edges
            .retain(|e| !doomed.contains(&e.from) && !doomed.contains(&e.to));
        // The edge lists now answer "is this node still linked?" per node,
        // where the old test scanned every edge for every node.
        self.reindex();
        let linked: Vec<NodeId> = self
            .adj
            .iter()
            .enumerate()
            .filter(|(_, at)| !at.is_empty())
            .map(|(id, _)| id as NodeId)
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
        self.reindex();
    }
}
