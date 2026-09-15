use super::*;

use crate::broker::recall::node::{Kind, Node, Rel};

fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-recall-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn seeded() -> Graph {
    let mut g = Graph::default();
    let turn = g.touch(Kind::Turn, "1", "you asked: x\ncrew answered: y", 10);
    let t = g.touch(Kind::Topic, "swarm", "swarm", 10);
    let f = g.touch(Kind::File, "src/a.rs", "src/a.rs", 10);
    g.link(turn, t, Rel::Mentions);
    g.link(turn, f, Rel::Mentions);
    g
}

#[test]
fn a_graph_written_out_and_replayed_is_the_same_graph() {
    let dir = temp("roundtrip");
    let path = path_at(&dir);
    let g = seeded();
    append(&path, &dump(&g));
    let back = load(&path);
    assert_eq!(back.len(), g.len());
    assert_eq!(back.edges().len(), g.edges().len());
    let id = back.find(Kind::Turn, "1").expect("the turn came back");
    assert_eq!(back.neighbors(id).count(), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_log_is_an_empty_graph_not_a_failure() {
    let dir = temp("cold");
    assert_eq!(load(&path_at(&dir)).len(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_torn_last_line_costs_that_record_and_nothing_else() {
    let dir = temp("torn");
    let path = path_at(&dir);
    append(&path, &dump(&seeded()));
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("{\"t\":\"n\",\"k\":\"top");
    std::fs::write(&path, text).unwrap();
    assert_eq!(load(&path).len(), 3, "a torn tail took the graph with it");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_log_under_the_cap_is_left_alone() {
    let dir = temp("small");
    let path = path_at(&dir);
    let mut g = seeded();
    append(&path, &dump(&g));
    assert!(!compact_if_needed(&path, &mut g));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_overgrown_log_is_pruned_and_rewritten_to_what_the_graph_now_holds() {
    let dir = temp("compact");
    let path = path_at(&dir);
    let mut g = Graph::default();
    for i in 0..(TURNS_KEPT + 20) as u64 {
        let turn = g.touch(Kind::Turn, &i.to_string(), &format!("turn {i}"), i);
        let t = g.touch(Kind::Topic, "swarm", "swarm", i);
        g.link(turn, t, Rel::Mentions);
    }
    // A log long enough to trip the threshold, whatever it was written as.
    let padding: Vec<String> = std::iter::repeat_n(
        crate::broker::recall::codec::node_line(&Node {
            kind: Kind::Topic,
            key: "swarm".into(),
            text: "swarm".into(),
            hits: 1,
            last_ms: 1,
        }),
        4001,
    )
    .collect();
    append(&path, &padding);
    assert!(compact_if_needed(&path, &mut g), "compaction did not run");
    let lines = std::fs::read_to_string(&path).unwrap().lines().count();
    assert!(lines < 4001, "the log was not rewritten ({lines} lines)");
    let back = load(&path);
    assert_eq!(back.count(Kind::Turn), TURNS_KEPT);
    assert!(
        back.find(Kind::Turn, "0").is_none(),
        "the oldest turn stayed"
    );
    std::fs::remove_dir_all(&dir).ok();
}
