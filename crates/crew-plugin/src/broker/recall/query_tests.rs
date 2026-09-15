use super::*;

use crate::broker::recall::node::Rel;

/// A graph of three turns: two about the swarm router, one about fonts.
fn seeded() -> Graph {
    let mut g = Graph::default();
    for (i, (text, words)) in [
        (
            "you asked: fix the router\ncrew answered: done",
            ["router", "swarm"],
        ),
        (
            "you asked: route a fan\ncrew answered: ok",
            ["router", "fanout"],
        ),
        (
            "you asked: pick a font\ncrew answered: lilex",
            ["font", "lilex"],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let turn = g.touch(Kind::Turn, &i.to_string(), text, i as u64 * 1000);
        let ids: Vec<NodeId> = words
            .iter()
            .map(|w| {
                let t = g.touch(Kind::Topic, w, w, i as u64 * 1000);
                g.link(turn, t, Rel::Mentions);
                t
            })
            .collect();
        g.link(ids[0], ids[1], Rel::With); // co-occurrence, as `ingest` writes it
    }
    g
}

#[test]
fn a_request_pulls_back_the_turns_about_the_same_thing_and_not_the_others() {
    let g = seeded();
    let hits = turns(&g, "the router is wrong again", &[], 4);
    let texts: Vec<&str> = hits
        .iter()
        .map(|h| g.node(h.id).unwrap().text.as_str())
        .collect();
    assert!(texts.iter().all(|t| !t.contains("font")), "{texts:?}");
    assert_eq!(texts.len(), 2, "both router turns should come back");
}

#[test]
fn a_second_hop_reaches_a_turn_that_shares_no_word_with_the_request() {
    // "fanout" names no word of the first turn; it reaches it through
    // `router`, which is the whole reason this is a graph.
    let g = seeded();
    let hits = turns(&g, "what happened with fanout", &[], 4);
    let texts: Vec<&str> = hits
        .iter()
        .map(|h| g.node(h.id).unwrap().text.as_str())
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("fix the router")),
        "{texts:?}"
    );
    assert!(
        texts[0].contains("route a fan"),
        "the direct hit must outrank the second hop: {texts:?}"
    );
}

#[test]
fn a_request_the_graph_has_never_seen_recalls_nothing() {
    let g = seeded();
    assert!(turns(&g, "what about the weather card", &[], 4).is_empty());
}

#[test]
fn a_turn_already_quoted_in_the_thread_is_not_quoted_again() {
    let g = seeded();
    let skip = vec!["fix the router".to_string()];
    let hits = turns(&g, "the router is wrong again", &skip, 4);
    assert_eq!(hits.len(), 1);
    assert!(g.node(hits[0].id).unwrap().text.contains("route a fan"));
}

#[test]
fn the_files_of_the_activated_turns_come_back_strongest_first() {
    let mut g = seeded();
    let turn = g.find(Kind::Turn, "0").unwrap();
    let f = g.touch(Kind::File, "src/route.rs", "src/route.rs", 1);
    g.link(turn, f, Rel::Mentions);
    assert_eq!(
        files(&g, "the router is wrong again", 4),
        vec!["src/route.rs"]
    );
}
