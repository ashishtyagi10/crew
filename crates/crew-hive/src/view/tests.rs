use super::*;
use crate::bus::{AgentId, HiveEvent};
use crate::graph::{AgentKind, ModelTier, TaskGraph, TaskId, TaskSpec, TaskState};
use crate::telemetry::Fleet;

#[test]
fn state_color_distinguishes_states() {
    assert_eq!(state_color(TaskState::Running), Rgb(0, 220, 140));
    assert_eq!(state_color(TaskState::Failed), Rgb(230, 80, 80));
    assert_ne!(
        state_color(TaskState::Pending),
        state_color(TaskState::Done)
    );
}

#[test]
fn mode_switches_at_threshold() {
    assert_eq!(mode_for_count(1), ViewMode::Constellation);
    assert_eq!(
        mode_for_count(HEATMAP_THRESHOLD - 1),
        ViewMode::Constellation
    );
    assert_eq!(mode_for_count(HEATMAP_THRESHOLD), ViewMode::Heatmap);
    assert_eq!(mode_for_count(5000), ViewMode::Heatmap);
}

#[test]
fn rgb_serde_roundtrip() {
    let c = Rgb(1, 2, 3);
    let j = serde_json::to_string(&c).unwrap();
    assert_eq!(serde_json::from_str::<Rgb>(&j).unwrap(), c);
}

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
fn constellation_chain_increases_x_by_depth() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0]), spec(2, &[1])]).unwrap();
    let c = constellation(&g, &Fleet::new());
    let by_id = |id: u64| c.nodes.iter().find(|n| n.task == TaskId(id)).unwrap();
    assert!(by_id(0).x < by_id(1).x && by_id(1).x < by_id(2).x);
    assert!(c.edges.contains(&Edge {
        from: TaskId(0),
        to: TaskId(1)
    }));
    assert!(c.edges.contains(&Edge {
        from: TaskId(1),
        to: TaskId(2)
    }));
    assert_eq!(c.nodes.len(), 3);
}

#[test]
fn constellation_node_state_reflects_fleet() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    let c = constellation(&g, &fleet);
    assert_eq!(c.nodes[0].state, TaskState::Running);
    assert_eq!(c.nodes[0].color, state_color(TaskState::Running));
}

#[test]
fn constellation_roots_share_layer_differ_in_y() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[])]).unwrap();
    let c = constellation(&g, &Fleet::new());
    let n0 = c.nodes.iter().find(|n| n.task == TaskId(0)).unwrap();
    let n1 = c.nodes.iter().find(|n| n.task == TaskId(1)).unwrap();
    assert_eq!(n0.x, n1.x);
    assert_ne!(n0.y, n1.y);
}

#[test]
fn heatmap_packs_row_major() {
    let mut fleet = Fleet::new();
    for i in 0..5u64 {
        fleet.apply(&HiveEvent::AgentSpawned {
            agent: AgentId(i),
            task: TaskId(i),
        });
    }
    let h = heatmap(&fleet, 2);
    assert_eq!(h.cols, 2);
    assert_eq!(h.rows, 3);
    assert_eq!(h.cells.len(), 5);
    let c4 = h.cells.iter().find(|c| c.agent == AgentId(4)).unwrap();
    assert_eq!((c4.row, c4.col), (2, 0));
}

#[test]
fn fleet_view_picks_constellation_when_small() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let mut fleet = Fleet::new();
    fleet.apply(&HiveEvent::AgentSpawned {
        agent: AgentId(0),
        task: TaskId(0),
    });
    assert!(matches!(
        fleet_view(&g, &fleet, 16),
        FleetView::Constellation(_)
    ));
}

#[test]
fn render_constellation_places_nodes_in_bounds() {
    let g = TaskGraph::new(vec![spec(0, &[]), spec(1, &[0])]).unwrap();
    let view = FleetView::Constellation(constellation(&g, &Fleet::new()));
    let cells = render_cells(&view, 40, 20);
    assert_eq!(cells.len(), 2);
    for c in &cells {
        assert!(c.col < 40 && c.row < 20);
        assert_eq!(c.ch, '●');
    }
    // the root (x=0) lands at col 0; the leaf (x=1.0) lands at the right edge
    let cols_sorted: Vec<u16> = {
        let mut v: Vec<u16> = cells.iter().map(|c| c.col).collect();
        v.sort_unstable();
        v
    };
    assert_eq!(cols_sorted[0], 0);
    assert_eq!(*cols_sorted.last().unwrap(), 39);
}

#[test]
fn render_heatmap_fits_small_grid_one_to_one() {
    let mut fleet = Fleet::new();
    for i in 0..4u64 {
        fleet.apply(&HiveEvent::AgentSpawned {
            agent: AgentId(i),
            task: TaskId(i),
        });
    }
    let view = FleetView::Heatmap(heatmap(&fleet, 2)); // 2 cols x 2 rows
    let cells = render_cells(&view, 40, 20);
    assert_eq!(cells.len(), 4);
    for c in &cells {
        assert!(c.col < 40 && c.row < 20);
        assert_eq!(c.ch, '■');
    }
}

#[test]
fn render_zero_viewport_is_empty() {
    let g = TaskGraph::new(vec![spec(0, &[])]).unwrap();
    let view = FleetView::Constellation(constellation(&g, &Fleet::new()));
    assert!(render_cells(&view, 0, 10).is_empty());
}
