use super::*;
use crate::chat::ChatPane;
use crew_plugin::Plugin;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
    }
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
fn card_has_header_then_indented_body() {
    let cells = message_cells(&[msg("planner", "hello")], 40, 10, 0, 0);
    assert_eq!(row_text(&cells, 0), format!("{GUTTER}planner"));
    assert_eq!(row_text(&cells, 1), " hello");
}

#[test]
fn cards_are_separated_by_a_blank_line() {
    let m = [msg("planner", "a"), msg("coder", "b")];
    let cells = message_cells(&m, 40, 10, 0, 0);
    assert_eq!(row_text(&cells, 2), ""); // spacer
    assert_eq!(row_text(&cells, 3), format!("{GUTTER}coder"));
}

#[test]
fn multiline_reply_renders_each_line() {
    let cells = message_cells(&[msg("coder", "one\ntwo")], 40, 10, 0, 0);
    assert_eq!(row_text(&cells, 1), " one");
    assert_eq!(row_text(&cells, 2), " two");
}

#[test]
fn fenced_code_renders_as_bordered_card() {
    let cells = message_cells(
        &[msg("coder", "fix:\n```rust\nlet x = 1;\n```")],
        40,
        10,
        0,
        0,
    );
    assert_eq!(row_text(&cells, 1), " fix:");
    assert_eq!(row_text(&cells, 2), " \u{256d}\u{2500} rust");
    assert_eq!(row_text(&cells, 3), " let x = 1;");
    assert_eq!(row_text(&cells, 4), " \u{2570}\u{2500}");
    // The code row sits on a bg different from the page background.
    let page = crew_theme::theme().page_bg;
    assert!(
        cells
            .iter()
            .any(|c| c.row == 3 && c.col > 0 && c.bg != page),
        "code should be on a dimmed card background"
    );
}

#[test]
fn header_tail_carries_latency_metadata() {
    let mut m = msg("coder", "done");
    m.meta = "4.2s".into();
    let cells = message_cells(&[m], 40, 10, 0, 0);
    assert!(
        row_text(&cells, 0).ends_with("\u{00b7} 4.2s"),
        "got: {}",
        row_text(&cells, 0)
    );
}

#[test]
fn handoff_sender_colours_each_name_separately() {
    let cells = message_cells(&[msg("planner \u{2192} coder", "x")], 40, 10, 0, 0);
    assert_eq!(
        row_text(&cells, 0),
        format!("{GUTTER}planner \u{2192} coder")
    );
    let muted = crew_theme::theme().text_muted;
    let cell_at = |col: u16| cells.iter().find(|c| c.row == 0 && c.col == col).unwrap();
    assert_ne!(cell_at(1).fg, muted, "planner keeps its agent colour");
    assert_ne!(cell_at(11).fg, muted, "coder keeps its agent colour");
}

#[test]
fn system_sender_is_muted_and_agents_are_not() {
    assert_eq!(sender_color("crew"), crew_theme::theme().text_muted);
    assert_ne!(sender_color("planner"), crew_theme::theme().text_muted);
}

#[test]
fn crew_message_uses_the_dotted_system_gutter() {
    let cells = message_cells(&[msg("crew", "hello")], 40, 10, 0, 0);
    assert_eq!(row_text(&cells, 0), "\u{2506}crew");
}

#[test]
fn agent_message_keeps_the_solid_gutter() {
    let cells = message_cells(&[msg("planner \u{2192} user", "hello")], 40, 10, 0, 0);
    assert_eq!(
        row_text(&cells, 0),
        format!("{GUTTER}planner \u{2192} user")
    );
}

#[test]
fn count_matches_rendered_lines_and_scroll_shows_older() {
    let m = [msg("a", "one"), msg("b", "two")];
    // 2 cards × (header + body) + 1 spacer = 5 lines.
    assert_eq!(card_line_count(&m, 40), 5);
    // A 2-row window scrolled 3 up from the bottom shows the first card.
    let cells = message_cells(&m, 40, 2, 0, 3);
    assert_eq!(row_text(&cells, 0), format!("{GUTTER}a"));
}

#[test]
fn top_row_offsets_and_width_clips() {
    let cells = message_cells(&[msg("planner", "wide text here")], 5, 4, 3, 0);
    assert!(cells.iter().all(|c| c.row >= 3 && c.col < 5));
}

#[test]
fn wide_glyphs_advance_two_columns() {
    // "中x": the wide glyph sits at its column and `x` lands TWO columns
    // later, so it can't overlap the glyph's second cell.
    let cells = message_cells(&[msg("a", "\u{4e2d}x")], 20, 4, 0, 0);
    let body: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == 1 && c.c != ' ')
        .map(|c| (c.col, c.c))
        .collect();
    let wide = body
        .iter()
        .find(|(_, c)| *c == '\u{4e2d}')
        .expect("wide glyph present");
    let x = body.iter().find(|(_, c)| *c == 'x').expect("x present");
    assert_eq!(x.0, wide.0 + 2, "got: {body:?}");
}

#[test]
fn header_line_shows_a_dim_chip_for_task_tagged_messages() {
    let m = Message {
        sender: "planner \u{2192} user".into(),
        text: "done".into(),
        ts: String::new(),
        meta: "task:2 \u{00b7} 0.0s".into(),
    };
    let line = header_line(&m, 0);
    let muted = crew_theme::theme().text_muted;
    let hash = line.iter().find(|c| c.c == '#').expect("chip # present");
    assert_eq!(hash.fg, muted, "chip # is muted");
    let id = line.iter().find(|c| c.c == '2').expect("chip id present");
    assert_eq!(id.fg, muted, "chip id is muted");
    let chars: String = line.iter().map(|c| c.c).collect();
    assert!(chars.contains("0.0s"), "latency must still render: {chars}");
    assert!(
        !chars.contains("task"),
        "tag must not leak into the header: {chars}"
    );
}

#[test]
fn header_line_has_no_chip_for_untagged_messages() {
    let mut m = msg("coder", "done");
    m.meta = "4.2s".into();
    let line = header_line(&m, 0);
    assert!(
        !line.iter().any(|c| c.c == '#'),
        "no task tag means no chip"
    );
}

#[test]
fn fade_t_ramps_with_message_age() {
    // Counting pass (now == 0) and unstamped messages render fully drawn.
    assert_eq!(fade_t("1000", 0), 1.0);
    assert_eq!(fade_t("", 5_000), 1.0);
    // A just-landed message starts faded and finishes after FADE_MS.
    assert_eq!(fade_t("5000", 5_000), 0.0);
    let mid = fade_t("5000", 5_000 + FADE_MS / 2);
    assert!(mid > 0.4 && mid < 0.6, "got: {mid}");
    assert_eq!(fade_t("5000", 5_000 + FADE_MS), 1.0);
}

fn test_pane(messages: Vec<Message>) -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut pane = ChatPane::new(plugin, "crew".into());
    pane.messages = messages;
    pane
}

#[test]
fn italic_cardcell_threads_through_to_cellview() {
    // `line_cells` is the per-row mapper `message_cells` maps over; a
    // hand-built italic cell pins that the flag survives to `CellView`
    // even before Task 4 wires a producer for it (markdown emphasis).
    let page = crew_theme::theme().page_bg;
    let line: CardLine = vec![CardCell {
        c: 'x',
        fg: (1, 2, 3),
        bold: false,
        italic: true,
        bg: None,
        link: None,
    }];
    let cells = line_cells(0, &line, 40, page);
    assert_eq!(cells.len(), 1);
    assert!(cells[0].italic, "italic must survive to the CellView");
}

#[test]
fn message_cells_is_a_thin_map_over_placed_lines() {
    use std::collections::HashSet;
    let pane = test_pane(vec![
        msg("planner", "one"),
        msg("coder", "two"),
        msg("crew", "three"),
    ]);
    let (cols, rows) = (40u16, 30u16);
    let top = pane.status_rows(cols, rows);
    let bottom = crate::chatinput::composer_rows(rows);
    let msg_rows = rows.saturating_sub(top + bottom);
    let cells = message_cells(&pane.messages, cols, msg_rows, top, pane.scroll);
    let placed = placed_lines(&pane, cols, rows);

    // Coverage independently derived from `placed_lines`, using the same
    // width/clip rules `message_cells` applies per row.
    let mut expected: HashSet<(u16, u16)> = HashSet::new();
    for (row, line) in &placed {
        let mut col = 0u16;
        for cell in line {
            let w = crate::chatwidth::char_w(cell.c) as u16;
            if w == 0 {
                continue;
            }
            if col + w > cols {
                break;
            }
            expected.insert((*row, col));
            col += w;
        }
    }
    let actual: HashSet<(u16, u16)> = cells.iter().map(|c| (c.row, c.col)).collect();
    assert_eq!(actual, expected);
    assert!(
        !actual.is_empty(),
        "sanity: the pane should render some cells"
    );
}

#[test]
fn msg_rows_budget_matches_view_math() {
    // Enough messages to overflow a modest pane, so `placed_lines` actually
    // gets clipped by the budget rather than trivially fitting everything.
    let messages: Vec<Message> = (0..30)
        .map(|i| msg("planner", &format!("line {i}")))
        .collect();
    let pane = test_pane(messages);
    let (cols, rows) = (40u16, 20u16);

    let budget = crate::chatplace::msg_rows_budget(&pane, cols, rows);
    let top = pane.status_rows(cols, rows);
    let placed = placed_lines(&pane, cols, rows);

    assert!(
        !placed.is_empty(),
        "sanity: overflowing pane still places lines"
    );
    assert!(
        placed.len() as u16 <= budget,
        "placed_lines returned more rows ({}) than the shared budget ({budget})",
        placed.len()
    );
    for (row, _) in &placed {
        assert!(
            *row >= top && *row < top + budget,
            "row {row} outside the message-area budget [{top}, {})",
            top + budget
        );
    }
}
