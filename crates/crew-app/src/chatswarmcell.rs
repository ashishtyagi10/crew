//! Placing the swarm block's text on the cell grid: one row, one colour per
//! char, never past the pane edge. Split from `chatswarmview` so the status
//! line and the task rows under it (`chatswarmrows`) put cells down the same
//! way — the pane-edge clamp is structural (`chatwidth::place_row`), not a
//! per-caller subtraction that two callers could get differently wrong.
use crew_render::CellView;

use crate::shimmer::Color;

/// Places `s` on `row` starting at `*col` in one colour — [`push_styled`]
/// over a plain string.
pub(crate) fn push_str(
    v: &mut Vec<CellView>,
    col: &mut u16,
    row: u16,
    s: &str,
    fg: Color,
    max_col: u16,
) {
    push_styled(v, col, row, s.chars().map(|c| (c, fg)), max_col, false);
}

/// Places styled chars on `row` starting at `*col`, advancing by display
/// width and never emitting a cell at or beyond `max_col` — delegated to
/// `chatwidth::place_row`, which advances by `char_w` and skips zero-width
/// marks, so `max_col` is enforced structurally.
pub(crate) fn push_styled(
    v: &mut Vec<CellView>,
    col: &mut u16,
    row: u16,
    chars: impl IntoIterator<Item = (char, Color)>,
    max_col: u16,
    bold: bool,
) {
    let bg = crew_theme::theme().page_bg;
    *col = crate::chatwidth::place_row(*col, max_col, chars, |x, c, fg| {
        v.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
            ..Default::default()
        });
    });
}
