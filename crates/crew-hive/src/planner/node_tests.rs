use serde_json::json;

use super::task_id;
use crate::graph::TaskId;
use crate::planner::{parse_plan, PlanError};

#[test]
fn an_id_is_a_whole_number_as_a_number_or_a_string_with_or_without_a_word_before_it() {
    for (v, n) in [
        (json!(0), 0),
        (json!(7), 7),
        (json!("1"), 1),
        (json!(" 2 "), 2),
        (json!("t1"), 1),
        (json!("T12"), 12),
        (json!("task-3"), 3),
        (json!("step_4"), 4),
    ] {
        assert_eq!(task_id(&v), Ok(n), "{v}");
    }
}

#[test]
fn anything_else_is_not_an_id() {
    for v in [
        json!("zero"),
        json!(""),
        json!("t"),
        json!("-1"),
        json!("+1"),
        json!("1a"),
        json!("t1b"),
        json!("t--1"),
        json!(-1),
        json!(1.5),
        json!(null),
        json!([1]),
    ] {
        assert!(task_id(&v).is_err(), "{v} must not read as an id");
    }
}

#[test]
fn string_ids_missing_deps_and_string_deps_make_the_same_graph_as_numbers() {
    let json = r#"[
        {"id": "t0", "title": "a", "prompt": "p"},
        {"id": "t1", "title": "b", "prompt": "q", "deps": null},
        {"id": "t2", "title": "c", "prompt": "r", "dependencies": ["t0", 1]}
    ]"#;
    let g = parse_plan(json).unwrap();
    assert_eq!(g.get(TaskId(0)).unwrap().deps, vec![]);
    assert_eq!(g.get(TaskId(1)).unwrap().deps, vec![]);
    assert_eq!(g.get(TaskId(2)).unwrap().deps, vec![TaskId(0), TaskId(1)]);
}

#[test]
fn two_ids_that_come_to_one_number_are_a_duplicate() {
    let json = r#"[{"id":"a1","title":"a","prompt":"p"},{"id":"b1","title":"b","prompt":"q"}]"#;
    assert!(matches!(parse_plan(json), Err(PlanError::Graph(_))));
}

#[test]
fn a_word_for_an_id_is_a_parse_error_that_names_it() {
    let json = r#"[{"id":0,"title":"a","prompt":"p","deps":["first"]}]"#;
    match parse_plan(json) {
        Err(PlanError::Parse(s)) => assert!(s.contains("\"first\""), "{s}"),
        other => panic!("expected a parse error, got {:?}", other.map(|g| g.len())),
    }
}

#[test]
fn deps_under_two_names_at_once_is_refused() {
    let json = r#"[{"id":0,"title":"a","prompt":"p","deps":[],"depends_on":[]}]"#;
    assert!(matches!(parse_plan(json), Err(PlanError::Parse(_))));
}
