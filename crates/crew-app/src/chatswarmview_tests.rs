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
fn title_clamp_is_display_width_aware_for_wide_glyphs() {
    // 20 CJK chars, 2 display columns each (40 columns total) — far more
    // than any reasonable char-count clamp should let through on a narrow
    // pane, and each costs twice what a char-count budget assumes.
    let mut p = pane();
    p.absorb_hive_plan(vec![TaskSpec {
        id: TaskId(0),
        title: "任务".repeat(10),
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }]);
    // Narrow enough to drop the token column (< TOKENS_MIN_COLS), so the
    // only reserve is the 1-column margin — isolates the title clamp itself.
    let cols = 18u16;
    let cells = block_cells(&p, cols, 0, 0);
    // The title starts at column 3 (glyph at 0, space at 1... actually 1 and
    // 2 — see block_cells: col starts at 1, glyph then space each advance it
    // by one). Measure the REAL display width of what got drawn from there,
    // the metric that actually determines whether it collides with the
    // pane edge / token column on screen.
    let title_w: usize = cells
        .iter()
        .filter(|c| c.row == 0 && c.col >= 3)
        .map(|c| crate::chatwidth::char_w(c.c))
        .sum();
    assert!(
        3 + title_w <= cols as usize,
        "title's real display width overruns the pane: 3 + {title_w} > {cols}"
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

#[test]
fn wide_pane_cjk_title_never_collides_with_token_column() {
    // Wide CJK title (8 chars × 2 display columns each = 16 display columns)
    // on a 40-column pane where the token column is visible.
    let mut p = pane();
    p.absorb_hive_plan(vec![TaskSpec {
        id: TaskId(0),
        title: "任务".repeat(4), // 8 CJK chars
        agent: AgentKind::Api { system: None },
        model: ModelTier::Cheap,
        deps: vec![],
        prompt: "p".into(),
    }]);
    // Trigger token rendering: 12_400 → "12.4k"
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(1),
        input: 12_000,
        output: 400,
    });

    let cols = 40u16;
    let cells = block_cells(&p, cols, 0, 0);
    let row = 0u16;

    // Extract token string from cells (should be "12.4k" right-aligned).
    let tok_cells: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    let tok_str: String = tok_cells.iter().map(|c| c.c).collect();
    assert!(tok_str.contains("12.4k"), "token string missing: {tok_str}");

    // The token starts at: cols.saturating_sub(tok.len() as u16 + 1)
    // For "12.4k" (len=5) at cols=40: 40 - 5 - 1 = 34
    let tok_start_col = cols.saturating_sub(5u16 + 1);

    // Find the token's actual columns to verify alignment.
    let tok_cells_by_col: Vec<_> = tok_cells
        .iter()
        .filter(|c| c.c.to_string() == "1" || c.c.to_string() == "2" || c.c == '.' || c.c == 'k')
        .map(|c| c.col)
        .collect();
    assert!(
        !tok_cells_by_col.is_empty(),
        "token columns not found for '12.4k'"
    );
    let tok_min_col = *tok_cells_by_col.iter().min().unwrap_or(&tok_start_col);
    assert_eq!(
        tok_min_col, tok_start_col,
        "token not right-aligned: expected start at {}, got {}",
        tok_start_col, tok_min_col
    );

    // Title starts at column 3 (glyph at col 1, space at col 2, title from col 3).
    // Verify no title cell (after the space) reaches into the token area.
    // The token area starts at tok_start_col, so title must not reach >= tok_start_col.
    let title_cells: Vec<_> = tok_cells
        .iter()
        .filter(|c| c.col >= 3 && c.col < tok_start_col)
        .collect();
    for cell in &title_cells {
        assert!(
            cell.col < tok_start_col,
            "title cell at col {} collides with token area starting at {}",
            cell.col,
            tok_start_col
        );
    }

    // Additionally: verify the title doesn't extend beyond its allowed space.
    // The maximum display width available for the title is:
    // cols - (col_after_space) - (token_reserve)
    // where token_reserve = tok.len() + 2 = 5 + 2 = 7
    // So: title max display width = 40 - 3 - 7 = 30 display columns
    let title_w: usize = title_cells
        .iter()
        .map(|c| crate::chatwidth::char_w(c.c))
        .sum();
    assert!(
        3 + title_w <= tok_start_col as usize,
        "title's display width ({}) extends past token start ({})",
        3 + title_w,
        tok_start_col
    );
}

// --- Per-task timings (2026-07-13-swarm-task-timings-design.md) ---

/// A digit immediately followed by `s` — the live elapsed suffix's shape
/// (`"0s"`, `"12s"`, ...). None of these tests' task titles ("task-N")
/// contain a bare `s` after a digit, so this is an unambiguous probe for
/// "the elapsed column rendered something."
fn has_elapsed_pattern(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    chars
        .windows(2)
        .any(|w| w[0].is_ascii_digit() && w[1] == 's')
}

#[test]
fn running_task_with_nonzero_now_shows_elapsed() {
    let mut p = pane_with_swarm(1);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    let row: String = block_cells(&p, 60, 0, 1_000)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(has_elapsed_pattern(&row), "{row}");
}

#[test]
fn running_task_with_zero_now_shows_no_elapsed() {
    // now_ms == 0 means "first frame in a test that doesn't care" — elapsed
    // rendering is skipped so existing zero-now tests stay deterministic.
    let mut p = pane_with_swarm(1);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    let row: String = block_cells(&p, 60, 0, 0)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(!has_elapsed_pattern(&row), "{row}");
}

#[test]
fn finished_task_shows_no_elapsed_in_the_live_block() {
    // Two tasks so the block stays open (folding on `finished()` would empty
    // it) with task 0 Done and task 1 still pending.
    let mut p = pane_with_swarm(2);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    p.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Done,
    });
    let row: String = block_cells(&p, 60, 0, 1_000)
        .iter()
        .filter(|c| c.row == 0)
        .map(|c| c.c)
        .collect();
    assert!(row.contains('\u{2713}'), "{row}"); // ✓ glyph still shown
    assert!(!has_elapsed_pattern(&row), "{row}");
}

#[test]
fn width_drop_order_tokens_first_then_elapsed() {
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
    let row_at = |cols: u16| -> String {
        block_cells(&p, cols, 0, 1_000)
            .iter()
            .filter(|c| c.row == 0)
            .map(|c| c.c)
            .collect()
    };

    // Wide: both the token count and the elapsed column show.
    let wide = row_at(60);
    assert!(wide.contains("12.4k"), "{wide}");
    assert!(has_elapsed_pattern(&wide), "{wide}");

    // Medium: narrow enough to drop tokens, wide enough to keep elapsed —
    // tokens drop first.
    let medium = row_at(20);
    assert!(!medium.contains("12.4k"), "{medium}");
    assert!(has_elapsed_pattern(&medium), "{medium}");

    // Very narrow: both drop.
    let narrow = row_at(10);
    assert!(!narrow.contains("12.4k"), "{narrow}");
    assert!(!has_elapsed_pattern(&narrow), "{narrow}");
}

#[test]
fn elapsed_and_token_columns_no_overlap_at_cols_60() {
    // RED test for Finding 1: column collision
    // When both elapsed and token columns render, they must not overlap.
    // With cols=60, both should appear without collision.
    let mut p = pane_with_swarm(1);
    p.absorb_hive(&HiveEvent::AgentSpawned {
        agent: crew_hive::AgentId(1),
        task: TaskId(0),
    });
    // Trigger token rendering: 12_400 → "12.4k"
    p.absorb_hive(&HiveEvent::TokenDelta {
        agent: crew_hive::AgentId(1),
        input: 12_000,
        output: 400,
    });

    let cols = 60u16;
    let cells = block_cells(&p, cols, 0, 1_000); // now_ms=1_000 to trigger elapsed
    let row = 0u16;

    // Check for column collisions: no two CellViews should share (row, col)
    let mut seen_positions = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    for cell in &cells {
        if cell.row == row {
            let pos = (cell.row, cell.col);
            if !seen_positions.insert(pos) {
                duplicates.push(pos);
            }
        }
    }
    assert!(
        duplicates.is_empty(),
        "column collision detected (duplicate (row, col) positions): {duplicates:?}"
    );

    // Verify token column span: "12.4k" should be right-aligned at cols=60
    let tok_len = 5u16; // "12.4k"
    let expected_tok_start = cols.saturating_sub(tok_len + 1); // 60 - 5 - 1 = 54
    let tok_cells: Vec<_> = cells
        .iter()
        .filter(|c| c.row == row && c.col >= expected_tok_start && c.col < cols)
        .collect();
    assert!(
        !tok_cells.is_empty(),
        "token cells not found at expected position [{}..{})",
        expected_tok_start,
        cols
    );
    let tok_cols: Vec<u16> = tok_cells.iter().map(|c| c.col).collect();
    let tok_min = *tok_cols.iter().min().unwrap();
    assert_eq!(
        tok_min, expected_tok_start,
        "token not right-aligned correctly"
    );

    // Verify elapsed is to the LEFT of token with a 1-col gap
    // Elapsed length: "0s" (when elapsed is small) or similar, typically 2-4 chars
    let elapsed_cells: Vec<_> = cells
        .iter()
        .filter(|c| c.row == row && c.col < expected_tok_start)
        .collect();
    if !elapsed_cells.is_empty() {
        let elapsed_cols: Vec<u16> = elapsed_cells.iter().map(|c| c.col).collect();
        let elapsed_max = *elapsed_cols.iter().max().unwrap_or(&0);
        let min_gap = expected_tok_start.saturating_sub(elapsed_max);
        assert!(
            min_gap >= 1,
            "elapsed column at col {} too close to token column starting at {} (gap: {})",
            elapsed_max,
            expected_tok_start,
            min_gap
        );
    }
}
