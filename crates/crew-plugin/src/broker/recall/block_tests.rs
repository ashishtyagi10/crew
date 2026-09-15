use super::*;

use crate::broker::recall::node::{Kind, Rel};

const NOW: u64 = 1_700_000_000_000;

fn seeded() -> Graph {
    let mut g = Graph::default();
    let turn = g.touch(
        Kind::Turn,
        "1",
        "you asked: why is the router slow\ncrew answered: it re-planned twice",
        NOW - 3 * 86_400_000,
    );
    let t = g.touch(Kind::Topic, "router", "router", NOW);
    let f = g.touch(Kind::File, "src/route.rs", "src/route.rs", NOW);
    g.link(turn, t, Rel::Mentions);
    g.link(turn, f, Rel::Mentions);
    g
}

#[test]
fn the_block_quotes_the_turn_with_how_long_ago_it_was_and_the_files() {
    let b = block(&seeded(), "the router again", &[], NOW, RECALL_CAP)
        .expect("a recall")
        .text;
    assert!(b.starts_with(HEAD), "{b}");
    assert!(b.contains("3d ago"), "{b}");
    assert!(b.contains("why is the router slow"), "{b}");
    assert!(b.contains("files that came up: src/route.rs"), "{b}");
    assert!(!b.contains('\t'));
}

#[test]
fn the_counts_report_the_turns_the_block_actually_quoted_and_how_far_back() {
    let r = block(&seeded(), "the router again", &[], NOW, RECALL_CAP).expect("a recall");
    assert_eq!(r.turns, 1);
    assert_eq!(
        r.oldest_ms,
        NOW - 3 * 86_400_000,
        "the reach is the oldest turn"
    );
    let quoted = r
        .text
        .lines()
        .filter(|l| l.starts_with("- ") && !l.contains("files that came up"))
        .count();
    assert_eq!(quoted, r.turns, "the count and the block disagree");
}

#[test]
fn a_cold_graph_renders_nothing_so_the_task_passes_through_untouched() {
    assert!(block(&Graph::default(), "anything", &[], NOW, RECALL_CAP).is_none());
}

#[test]
fn the_block_stays_inside_its_budget_and_says_where_it_cut() {
    let mut g = Graph::default();
    let long = "x".repeat(4000);
    let turn = g.touch(Kind::Turn, "1", &format!("you asked: router\n{long}"), NOW);
    let t = g.touch(Kind::Topic, "router", "router", NOW);
    g.link(turn, t, Rel::Mentions);
    let b = block(&g, "router", &[], NOW, 300).expect("a recall").text;
    assert!(b.chars().count() <= 300, "{} chars", b.chars().count());
    assert!(b.contains("clipped"), "a cut with no marker: {b}");
}

#[test]
fn a_turn_is_one_line_in_the_block_however_many_it_was_stored_as() {
    let b = block(&seeded(), "router", &[], NOW, RECALL_CAP)
        .unwrap()
        .text;
    let quoted = b.lines().nth(1).expect("a quoted turn");
    assert!(quoted.contains("why is the router slow"));
    assert!(
        quoted.contains("crew answered: it re-planned twice"),
        "{quoted}"
    );
}

#[test]
fn how_long_ago_is_told_in_the_coarsest_unit_that_is_still_true() {
    for (secs, want) in [
        (5u64, "just now"),
        (600, "10m ago"),
        (7200, "2h ago"),
        (86_400 * 3, "3d ago"),
        (86_400 * 30, "4w ago"),
    ] {
        assert_eq!(ago(NOW - secs * 1000, NOW), want);
    }
}
