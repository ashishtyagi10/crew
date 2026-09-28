//! The in-memory graph: nodes by `(kind, key)`, edges in one list, and per
//! node the positions of the edges it is an end of.
//!
//! The per-node list is a second copy of every edge's endpoints, and it earns
//! the invariant it costs. Recall walks two hops out of every seed on every
//! request, and a word like `test` sits on hundreds of edges; with the flat
//! list alone each step of that walk scanned EVERY edge in the file, so a
//! query cost grew with the whole history rather than with what it touched,
//! and so did finding the edge to strengthen on every insert. Now both cost
//! the node's own degree. The invariant — `adj[id]` holds exactly the edges
//! with `id` at either end — is kept by routing every write through
//! [`Graph::push_node`] and [`Graph::push_edge`], and by rebuilding it whole
//! when [`Graph::prune`] renumbers everything.
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
    /// Per node, the positions in `edges` it is an end of, in edge order.
    adj: Vec<Vec<usize>>,
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
        self.push_node(Node {
            kind,
            key,
            text: text.trim().to_owned(),
            hits: 1,
            last_ms: ms,
        })
    }

    /// Insert or strengthen `from -rel-> to`, returning the edge as it now
    /// stands so the caller can persist exactly what changed. Self-links are
    /// dropped (`None`): a topic co-occurring with itself is a loop that only
    /// ever inflates ranking.
    pub(crate) fn link(&mut self, from: NodeId, to: NodeId, rel: Rel) -> Option<Edge> {
        if from == to {
            return None;
        }
        let at = match self.edge_at(from, to, rel) {
            Some(at) => {
                let e = &mut self.edges[at];
                e.weight = e.weight.saturating_add(1);
                at
            }
            None => self.push_edge(Edge {
                from,
                to,
                rel,
                weight: 1,
            }),
        };
        Some(self.edges[at])
    }

    /// Restore a node exactly as the log recorded it (load path only).
    pub(crate) fn put(&mut self, n: Node) {
        match self.index.get(&(n.kind, n.key.clone())) {
            Some(&id) => self.nodes[id as usize] = n,
            None => {
                self.push_node(n);
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
        match self.edge_at(from, to, rel) {
            Some(at) => self.edges[at].weight = w,
            None => {
                self.push_edge(Edge {
                    from,
                    to,
                    rel,
                    weight: w,
                });
            }
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
        self.adj
            .get(id as usize)
            .into_iter()
            .flatten()
            .map(move |&at| {
                let e = &self.edges[at];
                (if e.from == id { e.to } else { e.from }, e.weight)
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

    /// The position of `from -rel-> to`, looked up from `from`'s own edges.
    fn edge_at(&self, from: NodeId, to: NodeId, rel: Rel) -> Option<usize> {
        self.adj.get(from as usize)?.iter().copied().find(|&at| {
            let e = &self.edges[at];
            e.from == from && e.to == to && e.rel == rel
        })
    }

    fn push_node(&mut self, n: Node) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.index.insert((n.kind, n.key.clone()), id);
        self.nodes.push(n);
        self.adj.push(Vec::new());
        id
    }

    fn push_edge(&mut self, e: Edge) -> usize {
        let at = self.edges.len();
        self.adj[e.from as usize].push(at);
        self.adj[e.to as usize].push(at);
        self.edges.push(e);
        at
    }

    /// Rebuild the key index and the per-node edge lists from `nodes` and
    /// `edges` — the one place ids change wholesale, so the one rebuild.
    fn reindex(&mut self) {
        self.index = self
            .nodes()
            .map(|(id, n)| ((n.kind, n.key.clone()), id))
            .collect();
        self.adj = vec![Vec::new(); self.nodes.len()];
        for (at, e) in self.edges.iter().enumerate() {
            self.adj[e.from as usize].push(at);
            self.adj[e.to as usize].push(at);
        }
    }
}

#[path = "prune.rs"]
mod prune;

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
