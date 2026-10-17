//! The swarm bar is drawn, not spelled. Split from `chatprog_tests`.
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

use super::{bar_art, bar_cells};

fn run(n: u64, settled: u64) -> crate::chat::ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = crate::chat::ChatPane::new(plugin, "crew".into());
    let task = |i| TaskSpec {
        id: TaskId(i),
        title: format!("task-{i}"),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
        specialty: String::new(),
        expertise: String::new(),
    };
    p.absorb_hive_plan((0..n).map(task).collect());
    for id in 0..settled {
        let state = TaskState::Done;
        p.absorb_hive(&HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state,
        });
    }
    p
}

/// No `█` or `░` survives into the frame; one capsule is painted in their
/// place, along the same row and as wide as the bar.
#[test]
fn a_live_bar_is_a_painted_capsule_over_blank_cells() {
    let _g = crate::app::theme_test_guard();
    let p = run(4, 2);
    let spelled = bar_cells(&p, 40, 7, 0);
    assert!(!spelled.is_empty(), "the run has a bar");
    let (cells, paint) = bar_art(&p, 40, 7, 0, 2.0);
    assert_eq!(cells.len(), spelled.len(), "the cells stay, for hit-tests");
    assert!(
        cells.iter().all(|c| c.c == ' '),
        "no glyph left to show through"
    );
    assert!(!paint.is_empty(), "the capsule is painted");
}

/// No run, no bar: nothing to blank and nothing painted.
#[test]
fn no_run_paints_nothing() {
    let _g = crate::app::theme_test_guard();
    let p = run(0, 0);
    let (cells, paint) = bar_art(&p, 40, 7, 0, 2.0);
    assert!(cells.is_empty() && paint.is_empty());
}

/// Whether `chatview::art` paints the swarm bar's capsule on `row` of a
/// `cols × rows` pane: the frame tests that read the bar's row ask this now
/// that the row's cells are blank.
pub(crate) fn bar_on(pane: &crate::chat::ChatPane, cols: u16, rows: u16, row: u16) -> bool {
    let (_, paint) = crate::chatview::art(pane, cols, rows, 2.0);
    let r = f32::from(row);
    paint
        .iter()
        .any(|p| p.y >= r && p.y + p.h <= r + 1.0 && p.w > 0.0)
}
