//! Recall: which of the remembered turns bear on the request in hand.
//!
//! Spreading activation, two hops (the weighing and the floor are in
//! [`super::score`]). The request's own topics and paths are the seeds;
//! activation flows along edges with a decay per hop, so a turn that shares a
//! word scores directly, and a turn that shares a word with a turn that shares
//! a word scores a little — which is the part a keyword search cannot do and
//! the reason this is a graph at all.
use super::graph::Graph;
use super::node::{Kind, NodeId};
use super::ruled::{is_ruled, quotes};
use super::score;

/// A hit: the node and how strongly the request activated it.
pub(crate) struct Hit {
    pub id: NodeId,
    pub score: f32,
}

/// The turns `text` should be reminded of, strongest first. `skip` holds the
/// requests already in front of the model (the live thread): remembering them
/// twice spends the budget on what it already has. A turn under the floor is
/// not one of them, however few the others are: a recall is a claim that the
/// turn bears on the request, and one shared common word does not make it.
pub(crate) fn turns(g: &Graph, text: &str, skip: &[String], max: usize) -> Vec<Hit> {
    let floor = score::floor();
    let mut hits: Vec<Hit> = score::activate(g, text)
        .into_iter()
        .filter(|(_, s)| *s >= floor)
        .filter(|(id, _)| g.node(*id).is_some_and(|n| n.kind == Kind::Turn))
        .filter(|(id, _)| {
            let stored = &g.node(*id).expect("filtered above").text;
            match is_ruled(stored) {
                // A correction quoting the request itself (noted as this very
                // turn was routed) or a message the thread already quotes.
                true => !skip
                    .iter()
                    .chain([&text.to_string()])
                    .any(|s| quotes(stored, s)),
                false => !skip.iter().any(|s| asked_of(stored) == s.trim()),
            }
        })
        .map(|(id, score)| Hit { id, score })
        .collect();
    // Corrections first, then by score: the cut below and a tight budget
    // both drop from the end, and what was ruled out is the one recollection
    // that changes what the model does next.
    let ruled = |id: NodeId| g.node(id).is_some_and(|n| is_ruled(&n.text));
    hits.sort_by(|a, b| {
        ruled(b.id)
            .cmp(&ruled(a.id))
            .then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| recency(g, b.id).cmp(&recency(g, a.id)))
    });
    hits.truncate(max);
    hits
}

/// The files `text` activated, strongest first — the "we were in these files"
/// line, which is often the useful half of a recall.
pub(crate) fn files(g: &Graph, text: &str, max: usize) -> Vec<String> {
    named(g, text, Kind::File, max)
}

/// The pages `text` activated: URLs a past turn cited, strongest first.
pub(crate) fn pages(g: &Graph, text: &str, max: usize) -> Vec<String> {
    named(g, text, Kind::Page, max)
}

/// The nodes of one kind that `text` activated, strongest first — those at
/// least one hop past the floor ([`score::trace`]), so a request that recalls
/// no turn does not name the files that its common words passed through.
fn named(g: &Graph, text: &str, kind: Kind, max: usize) -> Vec<String> {
    let trace = score::trace();
    let mut hits: Vec<(String, f32)> = score::activate(g, text)
        .into_iter()
        .filter(|(_, s)| *s >= trace)
        .filter_map(|(id, s)| {
            let n = g.node(id)?;
            (n.kind == kind).then(|| (n.text.clone(), s))
        })
        .collect();
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.truncate(max);
    hits.into_iter().map(|(p, _)| p).collect()
}

/// The newest turn whose request starts with `prefix` (empty matches any):
/// `(text, when)`. Time order, not relevance — this is what a pane opening
/// asks ("what were we doing?"), which is the one question memory answers
/// chronologically. A correction is not something that was being done, so
/// it is never the answer.
pub(crate) fn latest(g: &Graph, prefix: &str) -> Option<(String, u64)> {
    g.nodes()
        .filter(|(_, n)| n.kind == Kind::Turn && !is_ruled(&n.text))
        .filter(|(_, n)| asked_of(&n.text).starts_with(prefix))
        .max_by_key(|(_, n)| n.last_ms)
        .map(|(_, n)| (n.text.clone(), n.last_ms))
}

/// The request a stored turn opens with, for matching against the thread.
/// A turn is stored as `you asked: …\ncrew answered: …`; anything else is
/// matched whole, which simply never equals a request.
pub(crate) fn asked_of(text: &str) -> &str {
    text.lines()
        .next()
        .map(|l| l.strip_prefix("you asked: ").unwrap_or(l))
        .unwrap_or(text)
        .trim()
}

/// The answer half of a stored turn, empty when there is only a request.
pub(crate) fn answered_of(text: &str) -> &str {
    text.lines()
        .nth(1)
        .map(|l| l.strip_prefix("crew answered: ").unwrap_or(l))
        .unwrap_or("")
        .trim()
}

fn recency(g: &Graph, id: NodeId) -> u64 {
    g.node(id).map(|n| n.last_ms).unwrap_or(0)
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
