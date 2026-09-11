use super::reply;
use crew_hive::{AgentKind, ModelTier, TaskGraph, TaskId, TaskResult, TaskSpec};

fn spec(id: u64, title: &str, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: title.into(),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: title.into(),
        specialty: String::new(),
        expertise: String::new(),
    }
}

fn done(id: u64, output: &str) -> TaskResult {
    TaskResult {
        task: TaskId(id),
        output: output.into(),
        success: true,
    }
}

/// `gather` feeds `left` and `right`; nothing merges them — two sinks.
fn diamond() -> TaskGraph {
    TaskGraph::new(vec![
        spec(0, "gather", &[]),
        spec(1, "left", &[0]),
        spec(2, "right", &[0]),
    ])
    .unwrap()
}

/// `gather` then `merge` — one sink, whose reply is the answer.
fn chain() -> TaskGraph {
    TaskGraph::new(vec![spec(0, "gather", &[]), spec(1, "merge", &[0])]).unwrap()
}

#[test]
fn the_leads_closing_answer_is_the_turn_when_one_was_written() {
    let results = [done(0, "raw"), done(1, "l"), done(2, "r")];
    let r = reply(Some("Both agree: X.".into()), &diamond(), &results);
    assert_eq!(r.as_deref(), Some("Both agree: X."));
}

#[test]
fn one_sink_records_its_output_whole_and_not_the_tasks_before_it() {
    let results = [done(0, "the raw notes"), done(1, "the merged answer")];
    let r = reply(None, &chain(), &results);
    assert_eq!(r.as_deref(), Some("the merged answer"));
}

#[test]
fn two_sinks_record_each_under_its_title_and_skip_the_upstream_task() {
    let results = [
        done(0, "raw"),
        done(1, "left says A"),
        done(2, "right says B"),
    ];
    let r = reply(None, &diamond(), &results).unwrap();
    assert!(r.contains("## left\nleft says A"), "{r}");
    assert!(r.contains("## right\nright says B"), "{r}");
    assert!(!r.contains("gather") && !r.contains("raw"), "{r}");
    assert!(!r.starts_with('\n'), "trimmed: {r:?}");
}

#[test]
fn no_results_or_a_blank_sink_is_not_a_turn() {
    assert_eq!(reply(None, &chain(), &[]), None);
    let results = [done(0, "raw"), done(1, "   ")];
    assert_eq!(reply(None, &chain(), &results), None);
}
