use super::*;

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
