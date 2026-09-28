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

/// A xorshift, so the property below needs no dependency and replays the
/// same graph every run.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// What `neighbors` answered before there was an index: every edge scanned.
fn scan(g: &Graph, id: NodeId) -> Vec<(NodeId, u32)> {
    let other = |e: &Edge| match (e.from == id, e.to == id) {
        (true, _) => Some((e.to, e.weight)),
        (_, true) => Some((e.from, e.weight)),
        _ => None,
    };
    g.edges().iter().filter_map(other).collect()
}

fn assert_indexed(g: &Graph, when: &str) {
    for id in 0..g.len() as NodeId {
        let got: Vec<(NodeId, u32)> = g.neighbors(id).collect();
        assert_eq!(got, scan(g, id), "node {id} {when}");
    }
}

#[test]
fn neighbours_from_the_index_are_exactly_what_a_scan_of_every_edge_finds() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut g = Graph::default();
    let kinds = [Kind::Topic, Kind::File, Kind::Turn];
    let rels = [Rel::Mentions, Rel::With, Rel::Then];
    for i in 0..40u64 {
        g.touch(kinds[rng.below(3) as usize], &format!("n{i}"), "x", i);
    }
    for round in 0..400 {
        let (a, b) = (rng.below(40) as NodeId, rng.below(40) as NodeId);
        let rel = rels[rng.below(3) as usize];
        if round % 5 == 0 {
            // The load path: restored by key, over an edge or as a new one.
            let key = |id: NodeId| g.node(id).map(|n| (n.kind, n.key.clone())).unwrap();
            let (ka, kb) = (key(a), key(b));
            g.put_edge(ka, kb, rel, rng.below(9) as u32 + 1);
        } else {
            g.link(a, b, rel);
        }
    }
    let n = g.edges().len();
    assert!(n > 300, "too few edges to prove anything: {n}");
    assert!(
        g.edges().iter().any(|e| e.weight > 1),
        "no edge was ever re-linked"
    );
    assert_indexed(&g, "after linking");
    // Pruning renumbers every id; the index has to follow it.
    g.prune(4);
    assert!(g.edges().len() < n, "the prune dropped nothing");
    assert_indexed(&g, "after a prune");
}
