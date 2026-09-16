use super::*;

fn node(kind: Kind, key: &str) -> Node {
    Node {
        kind,
        key: key.into(),
        text: format!("text of {key}"),
        hits: 3,
        last_ms: 1_700_000_000_000,
    }
}

#[test]
fn a_node_round_trips_through_its_line_with_every_field_intact() {
    let n = node(Kind::Topic, "linecap");
    let Some(Rec::Node(back)) = decode(&node_line(&n)) else {
        panic!("a node line decoded as something else");
    };
    assert_eq!(back, n);
}

#[test]
fn an_edge_round_trips_naming_its_endpoints_by_kind_and_key() {
    let (from, to) = (node(Kind::Turn, "1700"), node(Kind::File, "src/nav.rs"));
    let e = Edge {
        from: 0,
        to: 1,
        rel: Rel::Mentions,
        weight: 2,
    };
    let line = edge_line(&e, &from, &to);
    assert!(line.contains("\"turn:1700\""), "{line}");
    let Some(Rec::Edge(f, t, rel, w)) = decode(&line) else {
        panic!("an edge line decoded as something else");
    };
    assert_eq!(f, (Kind::Turn, "1700".to_string()));
    assert_eq!(t, (Kind::File, "src/nav.rs".to_string()));
    assert_eq!((rel, w), (Rel::Mentions, 2));
}

#[test]
fn a_torn_or_foreign_line_decodes_to_nothing_instead_of_panicking() {
    for line in [
        "",
        "{\"t\":\"n\",\"k\":\"topi",        // a half-written append
        "{\"t\":\"n\",\"k\":\"decision\"}", // a kind this crew does not know
        "{\"t\":\"x\",\"k\":\"topic\"}",    // a record type it does not know
        "{\"t\":\"e\",\"f\":\"topic:a\"}",  // an edge missing its other end
    ] {
        assert!(decode(line).is_none(), "{line} decoded to something");
    }
}

#[test]
fn a_node_line_missing_its_counters_still_loads_with_a_first_sighting() {
    let Some(Rec::Node(n)) = decode("{\"t\":\"n\",\"k\":\"file\",\"key\":\"a.rs\"}") else {
        panic!("a sparse node line did not decode");
    };
    assert_eq!((n.hits, n.last_ms, n.text.as_str()), (1, 0, ""));
}

#[test]
fn every_kind_survives_the_round_trip_to_disk_and_back() {
    // A kind that cannot be read back is a kind that only exists until the
    // broker restarts — which is the one thing the graph is for.
    for kind in [Kind::Topic, Kind::File, Kind::Page, Kind::Turn] {
        let key = match kind {
            Kind::Page => "https://doc.rust-lang.org/std/",
            _ => "something",
        };
        let line = node_line(&node(kind, key));
        let Some(Rec::Node(back)) = decode(&line) else {
            panic!("{kind} did not decode: {line}");
        };
        assert_eq!(back.kind, kind, "{line}");
        assert_eq!(back.key, key, "{line}");
    }
}
