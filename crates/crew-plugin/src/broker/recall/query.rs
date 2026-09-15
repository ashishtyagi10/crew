//! Recall: which of the remembered turns bear on the request in hand.
//!
//! Spreading activation, two hops. The request's own topics and paths are the
//! seeds; activation flows along edges with a decay per hop, so a turn that
//! shares a word scores directly, and a turn that shares a word with a turn
//! that shares a word scores a little — which is the part a keyword search
//! cannot do and the reason this is a graph at all.
use std::collections::HashMap;

use super::extract;
use super::graph::Graph;
use super::node::{Kind, NodeId};

/// Activation kept per hop. Two hops at 0.45 means a second-hop turn needs
/// roughly five times the evidence of a first-hop one to outrank it.
const DECAY: f32 = 0.45;
/// Hops walked from the seeds.
const HOPS: usize = 2;
/// Edge weight past which more repetitions stop counting: a word used forty
/// times in one turn is not forty times the evidence.
const WEIGHT_CAP: u32 = 3;

/// A hit: the node and how strongly the request activated it.
pub(crate) struct Hit {
    pub id: NodeId,
    pub score: f32,
}

/// Seeds for `text`: the topic and path nodes the graph already has. A word
/// the graph has never seen contributes nothing — there is nothing to recall.
fn seeds(g: &Graph, text: &str) -> Vec<NodeId> {
    let mut out = Vec::new();
    for t in extract::topics(text) {
        if let Some(id) = g.find(Kind::Topic, &t) {
            out.push(id);
        }
    }
    for p in extract::paths(text) {
        if let Some(id) = g.find(Kind::File, &p) {
            out.push(id);
        }
    }
    out
}

/// Activation over the whole graph, seeded by `text`.
fn activate(g: &Graph, text: &str) -> HashMap<NodeId, f32> {
    let mut score: HashMap<NodeId, f32> = HashMap::new();
    let mut front: Vec<(NodeId, f32)> = seeds(g, text).into_iter().map(|id| (id, 1.0)).collect();
    for (id, s) in &front {
        *score.entry(*id).or_default() += s;
    }
    for _ in 0..HOPS {
        let mut next: Vec<(NodeId, f32)> = Vec::new();
        for (id, s) in front.drain(..) {
            for (to, w) in g.neighbors(id) {
                let add = s * DECAY * (w.min(WEIGHT_CAP) as f32);
                if add < 0.05 {
                    continue;
                }
                *score.entry(to).or_default() += add;
                next.push((to, add));
            }
        }
        front = next;
    }
    score
}

/// The turns `text` should be reminded of, strongest first. `skip` holds the
/// requests already in front of the model (the live thread): remembering them
/// twice spends the budget on what it already has.
pub(crate) fn turns(g: &Graph, text: &str, skip: &[String], max: usize) -> Vec<Hit> {
    let score = activate(g, text);
    let mut hits: Vec<Hit> = score
        .into_iter()
        .filter(|(id, _)| g.node(*id).is_some_and(|n| n.kind == Kind::Turn))
        .filter(|(id, _)| {
            let text = &g.node(*id).expect("filtered above").text;
            !skip.iter().any(|s| asked_of(text) == s.trim())
        })
        .map(|(id, score)| Hit { id, score })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| recency(g, b.id).cmp(&recency(g, a.id)))
    });
    hits.truncate(max);
    hits
}

/// The files `text` activated, strongest first — the "we were in these files"
/// line, which is often the useful half of a recall.
pub(crate) fn files(g: &Graph, text: &str, max: usize) -> Vec<String> {
    let score = activate(g, text);
    let mut hits: Vec<(String, f32)> = score
        .into_iter()
        .filter_map(|(id, s)| {
            let n = g.node(id)?;
            (n.kind == Kind::File).then(|| (n.text.clone(), s))
        })
        .collect();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(max);
    hits.into_iter().map(|(p, _)| p).collect()
}

/// The request a stored turn opens with, for matching against the thread.
/// A turn is stored as `you asked: …\ncrew answered: …`; anything else is
/// matched whole, which simply never equals a request.
fn asked_of(text: &str) -> &str {
    text.lines()
        .next()
        .map(|l| l.strip_prefix("you asked: ").unwrap_or(l))
        .unwrap_or(text)
        .trim()
}

fn recency(g: &Graph, id: NodeId) -> u64 {
    g.node(id).map(|n| n.last_ms).unwrap_or(0)
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
