use super::*;

#[test]
fn section_header_has_rule_and_legend() {
    let cells = section_header("SYS", 16, (70, 130, 140), (0, 255, 160), (8, 8, 16));
    // a horizontal rule is drawn on row 0
    assert!(cells.iter().any(|c| c.c == '─' && c.row == 0));
    // the legend sits on the same row in the title colour
    assert!(cells.iter().any(|c| c.c == 'S' && c.row == 0));
    // no box glyphs — this is a flat divider, not a card
    assert!(!cells
        .iter()
        .any(|c| matches!(c.c, '╭' | '╮' | '╰' | '╯' | '│')));
}

#[test]
fn section_header_too_narrow_is_empty() {
    assert!(section_header("x", 3, (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
}

#[test]
fn titled_card_has_corners_and_legend() {
    let cells = titled_card(20, 3, "~/code", (110, 110, 120), (0, 255, 160), (0, 0, 0));
    let has = |ch: char| cells.iter().any(|c| c.c == ch);
    assert!(has('╭') && has('╮') && has('╰') && has('╯'));
    // legend on the top border, in the title colour
    assert!(cells
        .iter()
        .any(|c| c.c == '~' && c.row == 0 && c.fg == (0, 255, 160)));
    // side borders on the interior row
    assert!(cells.iter().any(|c| c.c == '│' && c.row == 1 && c.col == 0));
}

#[test]
fn titled_card_too_small_is_empty() {
    assert!(titled_card(3, 3, "x", (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
    assert!(titled_card(20, 1, "x", (0, 0, 0), (0, 0, 0), (0, 0, 0)).is_empty());
}

#[test]
fn section_header_with_empty_title_is_solid_border() {
    let cells = section_header("", 16, (110, 110, 120), (0, 255, 160), (8, 8, 16));
    // All cells except column 0 should be '─' (the left corner is added by titled_card)
    let border_chars: Vec<char> = cells.iter().map(|c| c.c).collect();
    assert!(
        border_chars.iter().all(|c| *c == '─'),
        "empty title should produce solid border, got: {border_chars:?}"
    );
    // Specifically: no spaces in the border line (which would create a gap)
    assert!(
        !border_chars.contains(&' '),
        "empty title border must not contain spaces"
    );
}

#[test]
fn titled_card_with_empty_title_has_solid_top_border() {
    let cells = titled_card(20, 3, "", (110, 110, 120), (0, 255, 160), (0, 0, 0));
    let has = |ch: char| cells.iter().any(|c| c.c == ch);
    assert!(has('╭') && has('╮') && has('╰') && has('╯'));
    // Top border (row 0) should have only corners and '─' chars, no spaces
    let top_row: Vec<char> = cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect();
    assert!(
        top_row.iter().all(|c| matches!(c, '─' | '╭' | '╮')),
        "empty title top border should be solid, got: {top_row:?}"
    );
    assert!(
        !top_row.contains(&' '),
        "empty title top border must not contain spaces/gaps"
    );
}

/// Row-0 chars of a titled card, in column order, as one string.
fn top_row(cells: &[CellView]) -> String {
    let mut row: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
    row.sort_by_key(|c| c.col);
    row.iter().map(|c| c.c).collect()
}

#[test]
fn overlong_legend_ellipsizes_with_breathing_room_before_the_corner() {
    // 12 columns, so the title budget is 6. A long title must end in '…'
    // and keep ` ─╮` after it — never run flush into the corner.
    let cells = titled_card(
        12,
        3,
        "a-very-long-pane-title",
        (1, 1, 1),
        (2, 2, 2),
        (0, 0, 0),
    );
    assert_eq!(top_row(&cells), "╭─ a-ver… ─╮");
}

#[test]
fn exact_budget_legend_survives_untruncated() {
    let cells = titled_card(12, 3, "sixsix", (1, 1, 1), (2, 2, 2), (0, 0, 0));
    assert_eq!(top_row(&cells), "╭─ sixsix ─╮");
    assert!(!top_row(&cells).contains('…'));
}

#[test]
fn wide_glyph_legend_clips_on_a_cell_boundary() {
    // "日本語" is 6 columns wide; budget on a 10-col card is 4 → one wide
    // glyph (2) + '…' (1) = 3 columns of title, then the trailing ` ─`.
    let cells = titled_card(10, 3, "日本語", (1, 1, 1), (2, 2, 2), (0, 0, 0));
    let row = top_row(&cells);
    assert!(row.contains("… "), "wide legend must ellipsize: {row:?}");
    assert!(
        row.ends_with("─╮"),
        "rule must resume before the corner: {row:?}"
    );
    // The wide glyph advances two columns: no cell overlaps its shadow.
    let cols: Vec<u16> = {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == 0).map(|c| c.col).collect();
        v.sort_unstable();
        v
    };
    let mut uniq = cols.clone();
    uniq.dedup();
    assert_eq!(cols, uniq, "two cells share a column on the top border");
}

#[test]
fn no_room_for_a_legend_draws_a_solid_rule() {
    // 6 columns → zero title budget: the border stays solid rather than
    // showing a lone '…' jammed against the frame.
    let cells = titled_card(6, 3, "note", (1, 1, 1), (2, 2, 2), (0, 0, 0));
    assert_eq!(top_row(&cells), "╭────╮");
}

#[test]
fn titled_card_with_title_still_has_gapped_legend() {
    let cells = titled_card(20, 3, "WORK", (110, 110, 120), (0, 255, 160), (0, 0, 0));
    let top_row: Vec<char> = cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect();
    // With a title, the legend should have spaces around it (the old behavior)
    assert!(
        top_row.contains(&' '),
        "titled card with non-empty title should have gapped legend"
    );
    // And the title chars should be present
    assert!(
        top_row.contains(&'W'),
        "titled card should contain title characters"
    );
}
