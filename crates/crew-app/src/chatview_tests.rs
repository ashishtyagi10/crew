//! `link_at` resolves a markdown link's URL from a (row, col) in the
//! rendered message body — the click hit-test `clickopen` drives. Tests locate
//! the link text in the rendered `CellView`s rather than hardcoding layout
//! constants, so they stay independent of header/status-row geometry.
use super::*;
use crate::chat::ChatPane;
use crate::chatlayout::Message;
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
        })
        .collect();
    pane.absorb_hive_plan(tasks);
    let (cols, rows) = (30u16, 5u16);
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
