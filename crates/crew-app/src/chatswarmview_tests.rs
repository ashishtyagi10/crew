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
fn block_never_overdraws_the_composer_row_on_a_saturated_tiny_pane() {
    // rows=6 saturates the row budget for an 8-task swarm: with no messages
    // yet (plan absorbed, no reply landed), `chatview::cells`' empty-branch
    // call site used to hand block_cells a start row of 0 (the row budget
    // clamps to 0 via saturating_sub), drawing task rows straight through
    // the composer's row. crew-render's actual grid (celltext::fill_rich_text)
    // resolves overlapping cells last-write-wins per (row, col) — it does NOT
    // reliably let the composer's later-appended cells overdraw the block's,
    // since untouched columns in the composer's own cells (e.g. blank
    // interior past the prompt text) never touch that (row, col) at all.
    // Replicate that same last-write-wins merge here rather than
    // concatenating every cell's char, so the assertion reflects what's
    // actually drawn on screen.
    let (cols, rows) = (30u16, 6u16);
    let mut p = pane_with_swarm(8);
    // A short typed prompt (not the empty-input placeholder, which happens
    // to fill the whole row with hint text and would mask the bug): the
    // composer then only draws the `❯` glyph, a few typed chars, and a
    // caret — every other column on its row is untouched.
    p.input = "hi".into();
    assert!(
        p.messages.is_empty(),
        "plan absorption alone adds no message"
    );
    let composer_row = rows - crate::chatinput::composer_rows(&p.input, cols, rows);
    let cells = p.cells(cols, rows);
    // Replicate crew-render's actual grid merge (celltext::fill_rich_text
    // buckets cells into a rows×cols grid, later cells overwriting earlier
    // ones at the same (row, col)) rather than concatenating every cell's
    // char — a partially-overwritten block row (composer text punching a
    // hole through the middle of a task title) would otherwise dodge a
    // naive substring check while still leaking a corrupted glyph on screen.
    let mut grid: std::collections::HashMap<(u16, u16), char> = std::collections::HashMap::new();
    for c in &cells {
        grid.insert((c.row, c.col), c.c);
    }
    // The block cells the buggy (pre-fix) formula would hand to
    // `block_cells` for this pane/size — used only to know which (row, col)
    // positions a task title WOULD occupy, so we can check whether any of
    // them survives unmasked in the actual final grid at/after the
    // composer's first row.
    let block_top = rows
        .saturating_sub(crate::chatinput::composer_rows(&p.input, cols, rows))
        .saturating_sub(crate::chatswarmview::swarm_rows(&p, rows));
    let raw_block = crate::chatswarmview::block_cells(&p, cols, block_top, 0);
    let leaked: Vec<(u16, u16, char)> = raw_block
        .iter()
        .filter(|c| c.row >= composer_row)
        .filter(|c| grid.get(&(c.row, c.col)) == Some(&c.c))
        .map(|c| (c.row, c.col, c.c))
        .collect();
    assert!(
        leaked.is_empty(),
        "swarm-block cell(s) on/after the composer's first row ({composer_row}) survived \
         unmasked in the final grid: {leaked:?}"
    );
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
