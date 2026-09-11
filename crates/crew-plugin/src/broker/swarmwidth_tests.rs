//! The scheduler's width follows the plan, inside the clamp; the override
//! wins and is clamped too. Pure on `concurrency` so no test touches the
//! environment.
use super::*;

#[test]
fn a_one_task_plan_gets_the_floor_of_two() {
    assert_eq!(concurrency(1, None), MIN);
    assert_eq!(concurrency(0, None), MIN, "an empty plan is still not zero");
}

#[test]
fn a_twelve_wide_plan_is_held_to_eight() {
    assert_eq!(concurrency(12, None), MAX);
}

#[test]
fn a_plan_inside_the_clamp_runs_at_its_own_width() {
    for w in MIN..=MAX {
        assert_eq!(concurrency(w, None), w);
    }
}

#[test]
fn the_override_wins_over_the_plan_and_is_clamped_to_sixteen() {
    assert_eq!(concurrency(12, Some("3")), 3);
    assert_eq!(concurrency(1, Some(" 6 ")), 6, "spaces around the number");
    assert_eq!(concurrency(1, Some("40")), OVERRIDE_MAX);
    assert_eq!(concurrency(5, Some("1")), 1, "one is a legal serial run");
}

#[test]
fn a_junk_or_zero_override_is_ignored_and_the_plan_decides() {
    for raw in ["", "many", "0", "-2", "4x"] {
        assert_eq!(concurrency(5, Some(raw)), 5, "{raw:?}");
    }
}

#[test]
fn the_graph_reading_counts_only_tasks_with_no_deps() {
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};
    let spec = |id: u64, deps: &[u64]| TaskSpec {
        id: TaskId(id),
        title: format!("t{id}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Standard,
        deps: deps.iter().map(|d| TaskId(*d)).collect(),
        prompt: String::new(),
        specialty: String::new(),
        expertise: String::new(),
    };
    // Three roots and a merge: the first wave is three wide.
    let g = TaskGraph::new(vec![
        spec(0, &[]),
        spec(1, &[]),
        spec(2, &[]),
        spec(3, &[0, 1, 2]),
    ])
    .unwrap();
    assert_eq!(g.ready(&HashSet::new()).len(), 3);
    assert_eq!(concurrency(g.ready(&HashSet::new()).len(), None), 3);
}
