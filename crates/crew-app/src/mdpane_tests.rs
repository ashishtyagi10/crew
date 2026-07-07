use super::*;
use crew_render::CellView;
use std::path::PathBuf;

fn pane(source: &str) -> MdPane {
    MdPane::new(PathBuf::from("/tmp/doc.md"), source.to_string())
}

fn cell_at(cells: &[CellView], row: u16, col: u16) -> Option<&CellView> {
    cells.iter().find(|c| c.row == row && c.col == col)
}

fn row_text(cells: &[CellView], row: u16) -> String {
    row_text_before(cells, row, u16::MAX)
}

/// Same as `row_text`, but only columns strictly before `max_col` — lets a
/// test read just the source half without the divider/preview text after it
/// getting appended onto the same row.
fn row_text_before(cells: &[CellView], row: u16, max_col: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row && c.col < max_col)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn new_opens_at_top_of_source_with_no_scroll() {
    let p = pane("hello");
    assert_eq!(p.path, PathBuf::from("/tmp/doc.md"));
    assert_eq!(p.source, "hello");
    assert_eq!(p.active, Side::Source);
    assert_eq!(p.scroll_src, 0);
    assert_eq!(p.scroll_prev, 0);
}

#[test]
fn split_places_divider_at_expected_column() {
    let p = pane("hello");
    let cells = p.cells(41, 5);
    // left width = (41-1)/2 = 20, so the divider sits at column 20.
    for row in 0..5 {
        assert_eq!(
            cell_at(&cells, row, 20).map(|c| c.c),
            Some('\u{2502}'),
            "row {row} missing divider"
        );
    }
}

#[test]
fn source_side_shows_numbered_wrapped_lines() {
    let p = pane("hello\nworld");
    let cells = p.cells(41, 5);
    // Row 0's leading gutter space is overwritten by the active-side `▸`
    // indicator (source is active by default) — see
    // `indicator_marks_active_side_and_moves_after_tab` for that behavior.
    assert_eq!(row_text_before(&cells, 0, 20), "\u{25B8}  1 hello");
    assert_eq!(row_text_before(&cells, 1, 20), "   2 world");
}

#[test]
fn preview_side_renders_bold_span_from_markdown() {
    let p = pane("**x**");
    let cells = p.cells(41, 5);
    let bold_x = cells.iter().find(|c| c.c == 'x' && c.col > 20);
    assert!(bold_x.is_some(), "expected an 'x' cell on the preview side");
    assert!(bold_x.unwrap().bold, "expected the 'x' cell to be bold");
}

#[test]
fn source_scroll_beyond_end_clamps_to_last_page() {
    let lines: Vec<String> = (1..=50).map(|n| format!("line{n}")).collect();
    let mut p = pane(&lines.join("\n"));
    p.scroll_src = 10_000;
    let cells = p.cells(41, 5);
    // Clamped to the last full page: the final visible row is line 50.
    assert!(
        row_text(&cells, 4).contains("line50"),
        "expected last source line visible, got {:?}",
        row_text(&cells, 4)
    );
}

#[test]
fn preview_scroll_beyond_end_clamps_to_last_page() {
    let body: Vec<String> = (1..=50).map(|n| format!("para {n}\n")).collect();
    let mut p = pane(&body.join("\n"));
    p.scroll_prev = 10_000;
    let cells = p.cells(41, 5);
    let last_row_text = row_text(&cells, 4);
    assert!(
        last_row_text.contains("50"),
        "expected the last paragraph visible on the final row, got {last_row_text:?}"
    );
}

#[test]
fn tiny_and_zero_cols_never_panic() {
    let p = pane("hello **world**\nmore text here");
    for (cols, rows) in [(0, 0), (0, 5), (5, 0), (1, 1), (2, 3), (3, 1)] {
        let _ = p.cells(cols, rows);
        let _ = p.link_at(cols, rows, 0, 0);
    }
}

#[test]
fn link_at_hits_preview_link_and_misses_source_side() {
    let p = pane("[site](https://s.io)");
    let cells = p.cells(41, 5);
    // Find the 's' of the rendered link label "site" on the preview side.
    let link_cell = cells
        .iter()
        .find(|c| c.c == 's' && c.col > 20)
        .expect("expected link label cell on preview side");
    let hit = p.link_at(41, 5, link_cell.row, link_cell.col);
    assert_eq!(hit.as_deref(), Some("https://s.io"));

    // The same row on the source side must never resolve to a link.
    let miss = p.link_at(41, 5, 0, 5);
    assert_eq!(miss, None);
}

/// Same as `row_text_before`, but only columns strictly after `min_col` —
/// lets a test read just the preview half without the source text before
/// the divider getting prepended onto the same row.
fn row_text_after(cells: &[CellView], row: u16, min_col: u16) -> String {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.row == row && c.col > min_col)
        .map(|c| (c.col, c.c))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, c)| c).collect()
}

#[test]
fn indicator_marks_active_side_and_moves_after_tab() {
    let mut p = pane("hello");
    let cells = p.cells(41, 5);
    assert_eq!(
        cell_at(&cells, 0, 0).map(|c| c.c),
        Some('\u{25B8}'),
        "source is active by default"
    );
    assert_ne!(cell_at(&cells, 0, 21).map(|c| c.c), Some('\u{25B8}'));

    p.active = Side::Preview;
    let cells = p.cells(41, 5);
    assert_ne!(
        cell_at(&cells, 0, 0).map(|c| c.c),
        Some('\u{25B8}'),
        "source no longer marked"
    );
    assert_eq!(
        cell_at(&cells, 0, 21).map(|c| c.c),
        Some('\u{25B8}'),
        "preview now marked"
    );
}

#[test]
fn wheel_scrolls_the_side_under_the_cursor() {
    let mut p = pane("x");
    // cols=41 -> left_w=20, divider=20, right_start=21.
    p.scroll_wheel(41, Some(5), -1); // left of the divider -> source
    assert_eq!(p.scroll_src, 1);
    assert_eq!(p.scroll_prev, 0);
    p.scroll_wheel(41, Some(25), -1); // at/right of right_start -> preview
    assert_eq!(p.scroll_prev, 1);
}

#[test]
fn wheel_without_a_known_cursor_column_falls_back_to_the_active_side() {
    let mut p = pane("x");
    p.active = Side::Preview;
    p.scroll_wheel(41, None, -1);
    assert_eq!(p.scroll_prev, 1);
    assert_eq!(p.scroll_src, 0);
}

#[test]
fn preview_full_width_row_keeps_its_last_character() {
    // One long word (no spaces) so the markdown engine hard-wraps by char
    // count rather than at a word boundary — every full row fills the
    // preview width exactly, which is what exposes the dropped last column.
    let paragraph = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let p = pane(paragraph);
    // cols=41 -> left_w=20, divider=20, right_start=21, right_w=20.
    let cells = p.cells(41, 5);
    let raw: String = (0..5).map(|row| row_text_after(&cells, row, 20)).collect();
    // Every preview row is indented one column by `chatmd::map_lines`, so
    // strip spaces (the paragraph itself contains none) before comparing.
    let preview_text: String = raw.chars().filter(|c| *c != ' ').collect();
    assert_eq!(
        preview_text, paragraph,
        "expected every preview character to survive wrapping, got {raw:?}"
    );
}
