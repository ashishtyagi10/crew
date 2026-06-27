use super::*;
use std::collections::HashSet;

fn spec(id: u64, deps: &[u64]) -> TaskSpec {
    TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
    }
}

#[test]
fn new_accepts_valid_dag() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0, 1])]).unwrap();
    assert_eq!(g.len(), 3);
    assert!(!g.is_empty());
}

#[test]
fn new_rejects_duplicate_id() {
    let err = TaskGraph::new(vec![spec(0, &[]), spec(0, &[])]).unwrap_err();
    assert_eq!(err, GraphError::DuplicateId(TaskId(0)));
}

#[test]
fn new_rejects_missing_dep() {
    let err = TaskGraph::new(vec![spec(0, &[7])]).unwrap_err();
    assert_eq!(
        err,
        GraphError::MissingDep {
            task: TaskId(0),
            dep: TaskId(7)
        }
    );
}

#[test]
fn new_rejects_cycle() {
    let err = TaskGraph::new(vec![spec(0, &[1]), spec(1, &[0])]).unwrap_err();
    assert_eq!(err, GraphError::Cycle);
}

#[test]
fn ready_returns_roots_first() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[])]).unwrap();
    let done = HashSet::new();
    assert_eq!(g.ready(&done), vec![TaskId(0), TaskId(2)]);
}

#[test]
fn ready_unlocks_dependents_and_skips_done() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[0, 1])]).unwrap();
    let mut done = HashSet::new();
    done.insert(TaskId(0));
    assert_eq!(g.ready(&done), vec![TaskId(1)]); // 2 still blocked on 1; 0 is done
    done.insert(TaskId(1));
    assert_eq!(g.ready(&done), vec![TaskId(2)]);
    done.insert(TaskId(2));
    assert!(g.ready(&done).is_empty());
}

#[test]
fn get_and_serde_roundtrip() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    assert_eq!(g.get(TaskId(1)).unwrap().title, "t1");
    assert!(g.get(TaskId(9)).is_none());
    let json = serde_json::to_string(g.tasks()).unwrap();
    let back: Vec<TaskSpec> = serde_json::from_str(&json).unwrap();
    assert_eq!(back.len(), 2);
}

#[test]
fn diamond_dag_accepted_and_ready_correct() {
    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[0]),
        spec(2, &[0]),
        spec(3, &[1, 2]),
    ])
    .unwrap();
    let mut done = HashSet::new();
    assert_eq!(g.ready(&done), vec![TaskId(0)]);
    done.insert(TaskId(0));
    assert_eq!(g.ready(&done), vec![TaskId(1), TaskId(2)]);
    done.insert(TaskId(1));
    done.insert(TaskId(2));
    assert_eq!(g.ready(&done), vec![TaskId(3)]);
}
