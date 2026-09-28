//! How strongly a request lights each remembered node, and how strongly is
//! enough to be recalled.
//!
//! Spreading activation, two hops: the request's own topics and paths are the
//! seeds, and activation flows along edges with a decay per hop, so a turn
//! that shares a word scores directly and a turn one co-occurrence further on
//! scores a little. What decides whether any of it is worth a line in front
//! of the task is here too, because the two cannot be tuned apart: every
//! shared word once counted the same, there was no floor, and a request that
//! shared only `test` with fifty past turns carried four of them.
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
/// Turns a word may be in and still recall one on its own: 4% of the 400
/// kept (`graph::TURNS_KEPT`). Anything from 10 up lets the second hop in
/// [`floor`]'s list through (at 10 the floor is 0.142 against its 0.143);
/// sixteen leaves it room, and is a square, so the numbers come out exact.
const SPECIFIC: usize = 16;

/// What a word passes on per edge: `1/√d` for a word `d` turns mention.
///
/// A word in three turns says which three; a word in three hundred says
/// nothing about any one of them. The square root is the usual weight for a
/// node shared by `d` others: what a word hands out in total grows as `√d`,
/// not `d`, so a common word still reaches every turn it is in, faintly.
///
/// Not textbook IDF's `1/ln(e + d)`: that puts a word in one turn only three
/// times above a word in fifty (0.76 against 0.25), less than one hop of
/// spreading costs (×0.45, then the co-occurring word's own weight). A common word's DIRECT hit then outscores a rare
/// word's second hop, and no single floor keeps the second and drops the
/// first. The root puts them seven times apart (1 against 0.14).
fn rarity(d: usize) -> f32 {
    1.0 / (d.max(1) as f32).sqrt()
}

/// What a turn must score to be recalled: one direct mention of a word in
/// exactly [`SPECIFIC`] turns, `DECAY × rarity(16)` = 0.45 × ¼ = 0.1125. A
/// direct mention is worth `DECAY × rarity(d)`, so:
/// - a word in 1 turn gives 0.45, in 4 turns 0.225, in 16 turns 0.1125 —
///   each enough alone, which is what "rare" means here;
/// - a word in 17 turns gives 0.109, in 50 turns 0.064 — never enough alone,
///   by however many paths it arrives, because one word's share of a turn is
///   capped at its direct mention (see [`activate`]);
/// - two words each in 64 turns give 2 × 0.45/8 = 0.1125, so a turn that
///   shares two moderately common words with the request is enough;
/// - a word in 1 turn, one hop on through a word it came up with that is in
///   2 turns, gives 0.45 × 0.45 × 1/√2 = 0.143: a turn sharing no word with
///   the request can still come back, which is why this is a graph.
pub(crate) fn floor() -> f32 {
    DECAY * rarity(SPECIFIC)
}

/// The faintest contribution walked on, and what a file or page must score
/// to be named: one hop past the floor, 0.1125 × 0.45 = 0.051. It is exactly
/// what a file gets from a turn that only just cleared the floor, so the
/// files line stays about the turns worth recalling; and a word in 80 turns
/// or more falls under it before reaching even the turns that say it.
pub(crate) fn trace() -> f32 {
    floor() * DECAY
}

/// Seeds for `text`: the topic and path nodes the graph already has. A word
/// the graph has never seen contributes nothing — there is nothing to recall.
fn seeds(g: &Graph, text: &str) -> Vec<NodeId> {
    let topics = extract::topics(text).into_iter();
    let topics = topics.filter_map(|t| g.find(Kind::Topic, &t));
    let paths = extract::paths(text).into_iter();
    topics
        .chain(paths.filter_map(|p| g.find(Kind::File, &p)))
        .collect()
}

/// Activation over the whole graph, seeded by `text`.
///
/// Each seed is walked on its own, and what it gives any one node is capped
/// at what a direct mention gives. One word is one piece of evidence however
/// many paths it arrives by: without the cap, a word in twenty turns (0.10
/// each, under the floor) that three of them paired with a rarer word reached
/// those three a second time through it, and cleared the floor on the
/// strength of one common word. With it, the floor's arithmetic holds on any
/// graph, however its edges happen to fall.
pub(crate) fn activate(g: &Graph, text: &str) -> HashMap<NodeId, f32> {
    let trace = trace();
    let mut memo: HashMap<NodeId, f32> = HashMap::new();
    let mut score: HashMap<NodeId, f32> = HashMap::new();
    for seed in seeds(g, text) {
        *score.entry(seed).or_default() += 1.0;
        let cap = DECAY * passes(g, seed, &mut memo);
        let mut reached: HashMap<NodeId, f32> = HashMap::new();
        let mut front: Vec<(NodeId, f32)> = vec![(seed, 1.0)];
        for _ in 0..HOPS {
            let mut next: Vec<(NodeId, f32)> = Vec::new();
            for (id, s) in front.drain(..) {
                let out = s * DECAY * passes(g, id, &mut memo);
                for (to, w) in g.neighbors(id) {
                    let add = out * w.min(WEIGHT_CAP) as f32;
                    if to == seed || add < trace {
                        continue;
                    }
                    *reached.entry(to).or_default() += add;
                    next.push((to, add));
                }
            }
            front = next;
        }
        for (id, s) in reached {
            *score.entry(id).or_default() += s.min(cap);
        }
    }
    score
}

/// What a node passes on as it is walked through: a word its [`rarity`], on
/// every hop, and anything else all of it. A path is specific by itself (two
/// turns naming `crates/crew-hive/src/pricing.rs` share far more than two
/// saying `test`), and a turn has a handful of edges, never hundreds.
fn passes(g: &Graph, id: NodeId, memo: &mut HashMap<NodeId, f32>) -> f32 {
    if g.node(id).map(|n| n.kind) != Some(Kind::Topic) {
        return 1.0;
    }
    *memo.entry(id).or_insert_with(|| rarity(turns_of(g, id)))
}

/// The turns a node is joined to: a word's document frequency.
fn turns_of(g: &Graph, id: NodeId) -> usize {
    g.neighbors(id)
        .filter(|(n, _)| g.node(*n).is_some_and(|n| n.kind == Kind::Turn))
        .count()
}

#[cfg(test)]
#[path = "score_tests.rs"]
mod tests;
