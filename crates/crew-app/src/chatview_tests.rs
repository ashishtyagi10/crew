//! `link_at` resolves a markdown link's URL from a (row, col) in the
//! rendered message body — the click hit-test `clickopen` drives. Tests locate
//! the link text in the rendered `CellView`s rather than hardcoding layout
//! constants, so they stay independent of header/status-row geometry.
use super::*;
use crate::chat::ChatPane;
use crate::chatlayout::Message;
use crew_hive::{AgentKind, HiveEvent, ModelTier, TaskId, TaskSpec, TaskState};
use crew_plugin::Plugin;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    }
}

fn test_pane(messages: Vec<Message>) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.messages = messages;
    pane
}

/// The empty-state card must yield the rows the live-run surfaces claim.
///
/// `empty_cells` was handed `rows - bottom` as its max row while the status
/// line, queued indicator and bar budget `rows - (bottom + prog + queued)` —
/// so on an empty transcript with a live run (which is exactly when a run
/// starts: the plan lands before any reply) the onboarding text drew straight
/// through them. Last-write-wins made it a garbled interleave, not a clean
/// overdraw.
#[test]
fn empty_state_card_never_collides_with_a_live_runs_rows() {
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};
    let mut pane = test_pane(vec![]);
    pane.connected = true;
    let tasks = (0..4)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    pane.absorb_hive_plan(tasks);
    pane.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });

    for rows in 10..=24u16 {
        for cols in [40u16, 60, 80] {
            let cells = pane.cells(cols, rows);
            // Rows the live run owns: the status line, the bar, and the
            // queued indicator when showing. Nothing from the empty-state
            // card may share a row with them.
            let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
            let prog = crate::chatprog::progress_rows(&pane, cols);
            let block_max = rows.saturating_sub(bottom + prog);
            let line_rows: std::collections::HashSet<u16> =
                crate::chatswarmview::block_cells(&pane, cols, block_max.saturating_sub(1), 0)
                    .iter()
                    .map(|c| c.row)
                    .collect();
            if line_rows.is_empty() {
                continue; // pane too narrow for the line at all
            }
            // The empty card's own glyphs: everything the pane draws that the
            // status line and bar did not.
            let bar_row = rows.saturating_sub(bottom + prog);
            for c in &cells {
                let on_live = line_rows.contains(&c.row) || c.row == bar_row;
                if on_live {
                    // '❯' is the composer/card prompt cursor — it never belongs
                    // on a live-run row, whatever the row.
                    assert!(
                        c.c != '❯',
                        "empty-state card glyph {:?} landed on a live-run row \
                         {} at cols={cols} rows={rows}",
                        c.c,
                        c.row
                    );
                }
                if c.row == bar_row {
                    // The bar is block glyphs only, so a '·' here is a foreign
                    // surface. (On the status line the '·' is now the swarm
                    // parenthetical's own separator, so it's checked there by
                    // '❯' above, not by '·'.)
                    assert!(
                        c.c != '·',
                        "empty-state card bullet landed on the bar row \
                         {} at cols={cols} rows={rows}",
                        c.row
                    );
                }
            }
        }
    }
}

#[test]
fn empty_pane_swarm_block_never_draws_above_the_status_rows() {
    // A very short pane with a saturated (8-task) swarm block and no
    // messages yet: block_max.saturating_sub(swarm_rows) can bottom out at
    // (or below) 0, which — unfloored — lets the block's start row land
    // above `top`, overwriting the header/status rows.
    use crew_hive::{AgentKind, ModelTier, TaskId, TaskSpec};
    let mut pane = test_pane(vec![]);
    let tasks = (0..8)
        .map(|i| TaskSpec {
            id: TaskId(i),
            title: format!("task-{i}"),
            agent: AgentKind::Api { system: None },
            model: ModelTier::Cheap,
            deps: vec![],
            prompt: "p".into(),
            specialty: String::new(),
            expertise: String::new(),
        })
        .collect();
    pane.absorb_hive_plan(tasks);
    // Mark a task Running so the status line actually names `task-0` — with
    // nothing running the line reads "Working… 0/8" and the string "task-"
    // never appears anywhere in the pane, making the probe below vacuous.
    pane.absorb_hive(&HiveEvent::TaskStateChanged {
        task: TaskId(0),
        state: TaskState::Running,
    });
    // rows=5 leaves block_max comfortably above `top` for this fixture (no
    // roster, one progress-bar row, one swarm-line row), so the floor is
    // never actually exercised there. rows=3 is the genuine saturation case
    // (block_max == top == 1): the unclamped block_start bottoms out at 0,
    // one row above `top`.
    let (cols, rows) = (30u16, 3u16);
    let top = pane.status_rows(cols, rows);
    let out = cells(&pane, cols, rows);
    // Identify swarm-block cells by their distinctive content (task titles)
    // rather than by "any non-space cell", since the header/status rows
    // legitimately draw their own content in 0..top.
    let leaked: Vec<(u16, u16, char)> = out
        .iter()
        .filter(|c| c.row < top)
        .filter(|c| {
            let row_text: String = out.iter().filter(|o| o.row == c.row).map(|o| o.c).collect();
            row_text.contains("task-")
        })
        .map(|c| (c.row, c.col, c.c))
        .collect();
    assert!(
        leaked.is_empty(),
        "swarm block cell(s) landed above the status rows (top={top}): {leaked:?}"
    );
}

#[test]
fn link_at_resolves_the_clicked_link_and_misses_off_link() {
    // Link label "k" is rare enough not to collide with header/status text
    // ("crew", "1 msg", the connection dot) so the search below is unambiguous.
    let pane = test_pane(vec![msg("user", "see [k](https://x.io/p)")]);
    let (cols, rows) = (40u16, 20u16);
    let cells = cells(&pane, cols, rows);
    let k = cells
        .iter()
        .find(|c| c.c == 'k')
        .expect("link text 'k' rendered somewhere");
    assert_eq!(
        link_at(&pane, cols, rows, k.row, k.col).as_deref(),
        Some("https://x.io/p")
    );
    // Column 0 of the same row is the body's indentation cell — no link there.
    assert_eq!(link_at(&pane, cols, rows, k.row, 0), None);
}

#[test]
fn link_at_resolves_after_scrolling() {
    // Enough filler lines before the link message to overflow the row budget
    // (so scrolling actually moves the window), and exactly one filler
    // message after it, so the link's line stays a few lines shy of the
    // live-bottom edge — window() only drops lines from that edge as scroll
    // grows, so a scroll of 1 shifts the link's row without hiding it.
    let mut messages: Vec<Message> = (0..5)
        .map(|i| msg("planner", &format!("line {i}")))
        .collect();
    messages.push(msg("user", "see [k](https://x.io/p)"));
    messages.push(msg("planner", "tail"));
    let mut pane = test_pane(messages);
    let (cols, rows) = (40u16, 10u16);

    let before = cells(&pane, cols, rows);
    let k0 = before
        .iter()
        .find(|c| c.c == 'k')
        .expect("link visible before scroll");
    assert_eq!(
        link_at(&pane, cols, rows, k0.row, k0.col).as_deref(),
        Some("https://x.io/p")
    );

    pane.scroll = 1;
    let after = cells(&pane, cols, rows);
    let k1 = after
        .iter()
        .find(|c| c.c == 'k')
        .expect("link still visible after scrolling");
    assert_ne!(
        k1.row, k0.row,
        "scrolling should actually shift the link's row"
    );
    assert_eq!(
        link_at(&pane, cols, rows, k1.row, k1.col).as_deref(),
        Some("https://x.io/p"),
        "link must resolve at its shifted row after scrolling"
    );
}

#[test]
fn link_after_wide_glyphs_resolves_at_its_display_column() {
    // "中文 " advances the DISPLAY column by 4 (two CJK cells, 2 cols each)
    // plus 1 for the space, before the link starts — but only 3 Vec slots
    // (one per char). Raw `Vec` indexing by display column therefore misses
    // the link cell entirely; the fix must walk the line by display width.
    let pane = test_pane(vec![msg("user", "中文 [k](https://x.io/p)")]);
    let (cols, rows) = (40u16, 20u16);
    let cells_out = cells(&pane, cols, rows);
    let k = cells_out
        .iter()
        .find(|c| c.c == 'k')
        .expect("link text 'k' rendered somewhere");
    assert_eq!(
        link_at(&pane, cols, rows, k.row, k.col).as_deref(),
        Some("https://x.io/p"),
        "click on the link's own display column must resolve its URL"
    );
    // A click on the first CJK glyph's own cell must not resolve to the link.
    let cjk = cells_out
        .iter()
        .find(|c| c.c == '中')
        .expect("CJK glyph rendered");
    assert_eq!(link_at(&pane, cols, rows, cjk.row, cjk.col), None);
}

#[test]
fn compact_view_hides_link_on_a_clamped_line_but_keeps_it_on_the_visible_one() {
    // The link sits on the SECOND body line (a separate markdown paragraph —
    // a single `\n` is just a soft break within one line) — visible
    // normally, but clamped away once compact_view drops everything past
    // the first body line. The label "zzq" (not "k") avoids colliding with
    // the composer's own placeholder text ("...a task...", which itself
    // contains a 'k').
    let mut pane = test_pane(vec![msg("user", "see this:\n\n[zzq](https://x.io/p)")]);
    let (cols, rows) = (40u16, 20u16);

    let before = cells(&pane, cols, rows);
    let z = before
        .iter()
        .find(|c| c.c == 'z')
        .expect("link visible before compacting");
    assert_eq!(
        link_at(&pane, cols, rows, z.row, z.col).as_deref(),
        Some("https://x.io/p"),
        "link on the visible line hit-tests normally"
    );

    pane.compact_view = true;
    let after = cells(&pane, cols, rows);
    let after_text: String = after.iter().map(|c| c.c).collect();
    assert!(
        !after_text.contains("zzq"),
        "the link's line is clamped away in compact mode: {after_text:?}"
    );
    assert_eq!(
        link_at(&pane, cols, rows, z.row, z.col),
        None,
        "a link on a now-hidden line no longer hit-tests"
    );
}

#[test]
fn compact_view_keeps_link_on_the_first_body_line() {
    let mut pane = test_pane(vec![msg(
        "user",
        "see [k](https://x.io/p)\nmore detail here",
    )]);
    pane.compact_view = true;
    let (cols, rows) = (40u16, 20u16);
    let cells_out = cells(&pane, cols, rows);
    let k = cells_out
        .iter()
        .find(|c| c.c == 'k')
        .expect("link on the first (kept) body line still renders");
    assert_eq!(
        link_at(&pane, cols, rows, k.row, k.col).as_deref(),
        Some("https://x.io/p"),
        "a link on the visible clamped line still hit-tests"
    );
    let text: String = cells_out.iter().map(|c| c.c).collect();
    assert!(
        text.contains("\u{2026} +1"),
        "hidden-line suffix present: {text}"
    );
}

fn row_text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn queued_indicator_renders_directly_above_the_composer() {
    let mut pane = test_pane(vec![msg("planner", "hi")]);
    pane.queued.push_back("later".into());
    let (cols, rows) = (60u16, 20u16);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let indicator_row = rows - bottom - 1;

    let cells_out = cells(&pane, cols, rows);
    let text = row_text(&cells_out, indicator_row);
    assert!(
        text.contains("1 message queued"),
        "indicator row {indicator_row}: {text}"
    );
    assert!(text.contains("sends when the crew is idle"), "got: {text}");

    // Nothing else (message body, composer) bleeds into the indicator's row.
    let composer_row = rows - bottom;
    let composer_text = row_text(&cells_out, composer_row);
    assert_ne!(
        composer_text, text,
        "composer row is distinct from the indicator row"
    );
}

#[test]
fn queued_indicator_absent_when_queue_empty() {
    let pane = test_pane(vec![msg("planner", "hi")]);
    let (cols, rows) = (60u16, 20u16);
    let cells_out = cells(&pane, cols, rows);
    let text: String = cells_out.iter().map(|c| c.c).collect();
    assert!(
        !text.contains('\u{29d7}'),
        "no indicator glyph when idle/empty"
    );
}

#[test]
fn queued_indicator_never_draws_above_the_status_rows_on_a_squeezed_pane() {
    // Mirrors `empty_pane_swarm_block_never_draws_above_the_status_rows`:
    // `indicator_row` lacked the swarm block's `.max(top)` floor, so a
    // saturated roster (status_rows eating several rows for chip cards) on a
    // short pane can push the unclamped `rows - bottom - queued_rows` above
    // `top`, overdrawing the header/status rows.
    let mut pane = test_pane(vec![msg("planner", "hi")]);
    pane.agents = vec![
        crew_plugin::AgentInfo {
            name: "a".into(),
            role: String::new(),
            model: String::new(),
        },
        crew_plugin::AgentInfo {
            name: "b".into(),
            role: String::new(),
            model: String::new(),
        },
        crew_plugin::AgentInfo {
            name: "c".into(),
            role: String::new(),
            model: String::new(),
        },
    ];
    pane.queued.push_back("later".into());
    let (cols, rows) = (30u16, 5u16);
    let top = pane.status_rows(cols, rows);
    assert!(top > 0, "test setup must actually reserve status rows");

    let out = cells(&pane, cols, rows);
    // Identify indicator cells by the distinctive glyph, not "any cell above
    // top" — the header/status rows legitimately draw their own content there.
    let leaked: Vec<(u16, u16, char)> = out
        .iter()
        .filter(|c| c.row < top)
        .filter(|c| c.c == '\u{29d7}')
        .map(|c| (c.row, c.col, c.c))
        .collect();
    assert!(
        leaked.is_empty(),
        "queued indicator cell(s) landed above the status rows (top={top}): {leaked:?}"
    );
}

#[test]
fn queued_indicator_renders_on_the_empty_messages_branch_too() {
    let mut pane = test_pane(vec![]);
    pane.queued.push_back("a".into());
    pane.queued.push_back("b".into());
    let (cols, rows) = (60u16, 20u16);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let indicator_row = rows - bottom - 1;

    let cells_out = cells(&pane, cols, rows);
    let text = row_text(&cells_out, indicator_row);
    assert!(
        text.contains("2 messages queued"),
        "indicator row {indicator_row}: {text}"
    );
}

fn mid_run_pane() -> ChatPane {
    let mut pane = test_pane(vec![msg("crew", "planning the run")]);
    pane.absorb_hive_plan(
        (0..4)
            .map(|i| TaskSpec {
                id: TaskId(i),
                title: format!("task-{i}"),
                agent: AgentKind::Api { system: None },
                model: ModelTier::Cheap,
                deps: vec![],
                prompt: "p".into(),
                specialty: String::new(),
                expertise: String::new(),
            })
            .collect(),
    );
    for (id, state) in [(0, TaskState::Done), (1, TaskState::Running)] {
        pane.absorb_hive(&HiveEvent::TaskStateChanged {
            task: TaskId(id),
            state,
        });
    }
    pane
}

#[test]
fn progress_bar_renders_directly_above_the_composer() {
    let pane = mid_run_pane();
    let (cols, rows) = (60u16, 20u16);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let bar_row = rows - bottom - 1;

    let cells_out = cells(&pane, cols, rows);
    let text = row_text(&cells_out, bar_row);
    assert!(text.contains('\u{2588}'), "bar row {bar_row}: {text}");

    // The whole reason the rain was dropped: the indicator must claim its own
    // row, never overlay the composer's text.
    let composer_text = row_text(&cells_out, rows - bottom);
    assert!(
        !composer_text.contains('\u{2588}') && !composer_text.contains('\u{2591}'),
        "bar bled onto the composer row: {composer_text}"
    );
}

#[test]
fn progress_bar_and_queued_indicator_stack_without_colliding() {
    let mut pane = mid_run_pane();
    pane.queued.push_back("later".into());
    let (cols, rows) = (60u16, 20u16);
    let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
    let cells_out = cells(&pane, cols, rows);

    // Bar innermost (directly above the composer), queued indicator above it.
    let bar = row_text(&cells_out, rows - bottom - 1);
    let queued = row_text(&cells_out, rows - bottom - 2);
    assert!(bar.contains('\u{2588}'), "bar: {bar}");
    assert!(queued.contains("1 message queued"), "queued: {queued}");
}

/// Recovers the grid-merge helper from the deleted
/// `chatswarmview_tests::block_never_overdraws_the_composer_row_on_a_saturated_tiny_pane`
/// (`git show 2fa902f^:crates/crew-app/src/chatswarmview_tests.rs`) — the last
/// test that drove `pane.cells(cols, rows)` and replicated crew-render's
/// last-write-wins (row, col) merge. Its replacement in `chatswarmview_tests.rs`
/// (`cells_never_collide_or_leave_the_pane`) only calls `block_cells` in
/// isolation, so nothing exercises the status line, queued indicator, progress
/// bar and composer TOGETHER any more. Belongs here rather than in
/// `chatswarmview_tests.rs` because it's testing pane-level composition (four
/// separate widgets stacking without collision), not one widget's own geometry.
fn merge_grid(cells: &[CellView]) -> std::collections::HashMap<(u16, u16), char> {
    let mut grid = std::collections::HashMap::new();
    for c in cells {
        grid.insert((c.row, c.col), c.c);
    }
    grid
}

#[test]
fn status_line_queued_indicator_bar_and_composer_stack_without_colliding() {
    // A realistic mid-run pane: a message already in the transcript (so the
    // empty-transcript overlap — a known, out-of-scope pre-existing issue —
    // never comes into play), a 4-task plan with one task Running, and a
    // queued message, so all four bottom surfaces are showing at once.
    let mut pane = mid_run_pane();
    pane.queued.push_back("later".into());

    // Includes short panes: rows <= 9 used to pile every surface onto
    // `.max(top)`. `chatplace::grants` now drops what it cannot seat, so the
    // ones that DO draw still each own a row. Sizes where the status line is
    // dropped entirely are skipped by the guard below.
    for cols in [20u16, 40, 80] {
        for rows in [8u16, 10, 12, 20, 40] {
            let bottom = crate::chatinput::composer_rows(&pane.input, cols, rows);
            let prog_rows = crate::chatprog::progress_rows(&pane, cols);
            let queued_rows = crate::chatqueue::queued_rows(&pane);
            let swarm_rows = crate::chatswarmview::swarm_rows(&pane, cols);
            assert!(
                prog_rows > 0 && queued_rows > 0 && swarm_rows > 0,
                "fixture must actually exercise all three bottom surfaces at cols={cols} rows={rows}"
            );

            let top = pane.status_rows(cols, rows);
            let msg_rows = crate::chatplace::msg_rows_budget(&pane, cols, rows);
            let status_row = top + msg_rows;
            let indicator_row = rows - bottom - prog_rows - queued_rows;
            let bar_row = rows - bottom - prog_rows;
            let composer_row = rows - bottom;

            // Stacking order, message body toward composer: status line,
            // queued indicator, bar, composer — each on its own row.
            let ordered = [status_row, indicator_row, bar_row, composer_row];
            for w in ordered.windows(2) {
                assert!(
                    w[0] < w[1],
                    "rows out of order at cols={cols} rows={rows}: {ordered:?}"
                );
            }

            // Replicate crew-render's actual last-write-wins (row, col) grid
            // merge rather than concatenating every cell's char — a
            // partially-overwritten row (one surface punching a hole through
            // another) could dodge a naive substring check while still
            // leaking a corrupted glyph on screen.
            let grid = merge_grid(&cells(&pane, cols, rows));
            let row_text = |row: u16| -> String {
                (0..cols).filter_map(|col| grid.get(&(row, col))).collect()
            };

            let status_text = row_text(status_row);
            assert!(
                status_text.contains("task-1"),
                "status line missing/overwritten at cols={cols} rows={rows}: {status_text:?}"
            );

            let indicator_text = row_text(indicator_row);
            assert!(
                indicator_text.contains("queued"),
                "queued indicator missing/overwritten at cols={cols} rows={rows}: {indicator_text:?}"
            );

            let bar_text = row_text(bar_row);
            assert!(
                bar_text.contains('\u{2588}') || bar_text.contains('\u{2591}'),
                "progress bar missing/overwritten at cols={cols} rows={rows}: {bar_text:?}"
            );

            // None of the three surfaces above bleed onto the composer's row.
            let composer_text = row_text(composer_row);
            assert!(
                !composer_text.contains('\u{2588}')
                    && !composer_text.contains('\u{2591}')
                    && !composer_text.contains("queued")
                    && !composer_text.contains("task-1"),
                "a surface bled onto the composer row at cols={cols} rows={rows}: {composer_text:?}"
            );
        }
    }
}

#[test]
fn compact_view_shows_the_header_chip_end_to_end() {
    let mut pane = test_pane(vec![msg("planner", "hi")]);
    let (cols, rows) = (60u16, 20u16);
    let before = row_text(&cells(&pane, cols, rows), 0);
    assert!(
        !before.contains("compact"),
        "no chip when compact_view is off: {before}"
    );

    pane.compact_view = true;
    let after = row_text(&cells(&pane, cols, rows), 0);
    assert!(after.contains("compact"), "chip missing: {after}");
}
