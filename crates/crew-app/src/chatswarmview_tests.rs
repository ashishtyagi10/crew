use super::*;
use crate::chat::ChatPane;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn pane() -> ChatPane {
    // An idle child stands in for the broker; only pane state is under test.
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    ChatPane::new(plugin, "crew".into())
}

fn pane_with_swarm(n: u64) -> ChatPane {
    let mut p = pane();
    let tasks = (0..n)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
        })
        .collect();
    p.absorb_hive_plan(tasks);
    p
}

#[test]
fn no_swarm_no_rows() {
    let p = pane();
    assert_eq!(swarm_rows(&p, 40), 0);
    assert!(block_cells(&p, 80, 5, 0).is_empty());
}

#[test]
fn one_row_per_task_capped_at_eight() {
    assert_eq!(swarm_rows(&pane_with_swarm(3), 40), 3);
    assert_eq!(swarm_rows(&pane_with_swarm(20), 40), 8);
}

#[test]
fn block_rows_render_titles_with_state_glyphs() {
    let mut p = pane_with_swarm(2);
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    let cells = block_cells(&p, 80, 10, 0);
    let row10: String = cells.iter().filter(|c| c.row == 10).map(|c| c.c).collect();
    let row11: String = cells.iter().filter(|c| c.row == 11).map(|c| c.c).collect();
    assert!(row10.contains('✓') && row10.contains("task-0"), "{row10}");
    assert!(row11.contains("task-1"), "{row11}");
}

#[test]
fn token_counts_right_aligned_on_wide_panes_dropped_on_narrow() {
    let mut p = pane_with_swarm(1);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(1),
        input: 12_000,
        output: 400,
    });
    let wide: String = block_cells(&p, 60, 0, 0)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(wide.contains("12.4k"), "{wide}");
    let narrow: String = block_cells(&p, 18, 0, 0)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(!narrow.contains("12.4k"), "{narrow}");
}
