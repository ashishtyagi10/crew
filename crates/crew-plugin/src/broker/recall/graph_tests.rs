use super::*;

fn topic(g: &mut Graph, w: &str) -> NodeId {
    g.touch(Kind::Topic, w, w, 10)
}

#[test]
fn two_spellings_of_one_topic_are_one_node_that_counts_both() {
    let mut g = Graph::default();
    let a = g.touch(Kind::Topic, "Linecap", "Linecap", 10);
    let b = g.touch(Kind::Topic, " linecap ", "linecap", 20);
    assert_eq!(a, b, "case and padding made a second node");
    assert_eq!(g.len(), 1);
    let n = g.node(a).unwrap();
    assert_eq!((n.hits, n.last_ms), (2, 20));
    assert_eq!(n.text, "Linecap", "the first spelling is the one shown");
}

#[test]
fn a_repeated_link_gains_weight_instead_of_a_duplicate_edge() {
    let mut g = Graph::default();
    let (a, b) = (topic(&mut g, "swarm"), topic(&mut g, "router"));
    g.link(a, b, Rel::With);
    let e = g.link(a, b, Rel::With).expect("a real link");
    assert_eq!(g.edges().len(), 1);
    assert_eq!(e.weight, 2);
}

#[test]
fn a_self_link_is_refused_because_it_only_ever_inflates_ranking() {
    let mut g = Graph::default();
    let a = topic(&mut g, "swarm");
    assert!(g.link(a, a, Rel::With).is_none());
    assert!(g.edges().is_empty());
}

#[test]
fn neighbours_answer_from_both_ends_of_an_edge() {
    let mut g = Graph::default();
    let turn = g.touch(Kind::Turn, "1", "you asked: x", 10);
    let t = topic(&mut g, "swarm");
    g.link(turn, t, Rel::Mentions);
    assert_eq!(g.neighbors(turn).collect::<Vec<_>>(), vec![(t, 1)]);
    assert_eq!(
        g.neighbors(t).collect::<Vec<_>>(),
        vec![(turn, 1)],
        "a topic must lead back to its turns, or recall walks one way only"
    );
}

#[test]
fn an_edge_whose_endpoint_was_never_loaded_is_dropped_not_misdirected() {
    let mut g = Graph::default();
    g.put(Node {
        kind: Kind::Topic,
        key: "swarm".into(),
        text: "swarm".into(),
        hits: 1,
        last_ms: 1,
    });
    g.put_edge(
        (Kind::Topic, "swarm".into()),
        (Kind::File, "gone.rs".into()),
        Rel::Mentions,
        4,
    );
    assert!(g.edges().is_empty(), "a dangling edge was kept");
}

#[test]
fn pruning_keeps_the_newest_turns_and_takes_their_orphans_with_the_rest() {
    let mut g = Graph::default();
    for i in 0..4u64 {
        let turn = g.touch(Kind::Turn, &i.to_string(), &format!("turn {i}"), i * 1000);
        let t = g.touch(Kind::Topic, &format!("topic{i}"), "t", i * 1000);
        g.link(turn, t, Rel::Mentions);
    }
    g.prune(2);
    let kept: Vec<String> = g
        .nodes()
        .filter(|(_, n)| n.kind == Kind::Turn)
        .map(|(_, n)| n.key.clone())
        .collect();
    assert_eq!(kept, vec!["2".to_string(), "3".to_string()]);
    assert_eq!(g.count(Kind::Topic), 2, "topics of dropped turns stayed");
    // Ids were rebuilt: every edge must still point at the pair it named.
    for e in g.edges() {
        let (from, to) = (g.node(e.from).unwrap(), g.node(e.to).unwrap());
        assert_eq!(from.kind, Kind::Turn);
        assert_eq!(to.text, "t", "an edge survived pointing at the wrong node");
    }
    assert!(g.find(Kind::Turn, "3").is_some(), "the index went stale");
}
