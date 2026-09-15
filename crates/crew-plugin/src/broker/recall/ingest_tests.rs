use super::*;

use crate::broker::recall::node::{Kind, Rel};
use crate::broker::recall::store;

#[test]
fn one_turn_becomes_a_turn_node_joined_to_its_topics_and_its_files() {
    let mut r = Recall::default();
    r.record(
        "why does the swarm re-plan in crates/crew-hive/src/planner/mod.rs",
        "because the planner replan budget is one",
    );
    assert_eq!(r.stats().0, 1, "no turn recorded");
    assert!(r.stats().1 > 0, "no topics recorded");
    assert_eq!(r.stats().2, 1, "the path was not recorded as a file");
    let id = r.prev.expect("the turn is the spine's head");
    let joined: Vec<Kind> =
        r.g.neighbors(id)
            .filter_map(|(to, _)| r.g.node(to).map(|n| n.kind))
            .collect();
    assert!(
        joined.contains(&Kind::File),
        "the turn did not join its file"
    );
    assert!(
        joined.contains(&Kind::Topic),
        "the turn did not join a topic"
    );
}

#[test]
fn an_unfinished_turn_is_not_remembered() {
    let mut r = Recall::default();
    r.record("a question", "");
    r.record("", "an answer");
    assert_eq!(r.nodes(), 0);
}

#[test]
fn consecutive_turns_are_joined_into_the_sessions_spine() {
    let mut r = Recall::default();
    r.record("first about routing", "done");
    let first = r.prev.expect("a first turn");
    r.record("second about fonts", "done");
    let second = r.prev.expect("a second turn");
    assert!(
        r.g.edges()
            .iter()
            .any(|e| e.from == first && e.to == second && e.rel == Rel::Then),
        "the turns were not joined in order"
    );
}

#[test]
fn a_long_answer_is_clipped_before_it_reaches_the_graph() {
    let mut r = Recall::default();
    r.record("about routing", &"y".repeat(ANSWER_KEPT * 4));
    let text = &r.g.node(r.prev.unwrap()).unwrap().text;
    assert!(
        text.chars().count() < ANSWER_KEPT * 2,
        "{} chars",
        text.len()
    );
    assert!(text.contains("clipped"));
}

#[test]
fn recording_the_same_topic_twice_bumps_it_rather_than_forking_it() {
    let mut r = Recall::default();
    r.record("the router decides the shape", "yes");
    let before = r.stats().1;
    r.record("the router again", "yes");
    assert_eq!(r.stats().1, before, "a repeat topic forked a second node");
    let id = r.g.find(Kind::Topic, "router").expect("the topic");
    assert_eq!(r.g.node(id).unwrap().hits, 2);
}

#[test]
fn only_what_changed_is_appended_not_the_whole_graph() {
    let dir = std::env::temp_dir().join(format!("crew-recall-append-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let mut r = Recall::open_at(&dir);
    for i in 0..6 {
        r.record(&format!("turn {i} about routing"), "done");
    }
    let lines = std::fs::read_to_string(store::path_at(&dir))
        .unwrap()
        .lines()
        .count();
    // Six turns, a handful of records each — a whole-graph dump per turn
    // would already be past a hundred lines here.
    assert!(lines < 80, "{lines} lines written for six turns");
    assert_eq!(store::load(&store::path_at(&dir)).count(Kind::Turn), 6);
    std::fs::remove_dir_all(&dir).ok();
}
