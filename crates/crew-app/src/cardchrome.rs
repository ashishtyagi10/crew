//! A pane card's chrome: the corner brackets that mark focus, and the close
//! and minimize buttons riding its top border.
//!
//! Split from [`crate::panecard`] for the line cap, along the line between the
//! card and the marks laid on top of it.
use crate::layout::Rect;
use crew_render::CellView;

/// Longest a focus bracket grows, in cells down each edge from a corner. Short
/// on purpose: this is a corner mark, not a second border.
pub(crate) const BRACKET_MAX: u16 = 4;

/// Bracket length for a card `rows` tall at eased progress `t`. Bounded to a
/// third of the card so a two-row strip thumbnail can't have its whole side
/// lit up and read as focused-everywhere.
pub(crate) fn bracket_len(rows: u16, t: f32) -> u16 {
    let room = rows.saturating_sub(2) / 3;
    let max = room.min(BRACKET_MAX);
    (max as f32 * t.clamp(0.0, 1.0)).round() as u16
}

/// Recolour the border cells forming the four corner brackets. Vertical only:
/// the top edge carries the legend and the `[-][x]` buttons, and a bracket that
/// overwrote either would be trading information for decoration.
pub(crate) fn draw_brackets(v: &mut [CellView], cols: u16, rows: u16, t: f32) {
    let n = bracket_len(rows, t);
    if n == 0 || cols < 2 || rows < 4 {
        return;
    }
    let accent = crate::palette::accent();
    let (left, right) = (0, cols - 1);
    for i in 0..n {
        for (col, row) in [
            (left, 1 + i),
            (right, 1 + i),
            (left, rows - 2 - i),
            (right, rows - 2 - i),
        ] {
            if let Some(cell) = v.iter_mut().find(|c| c.col == col && c.row == row) {
                cell.fg = accent;
                cell.bold = true;
            }
        }
    }
}

/// Narrowest card (in cells, border included) that carries the border
/// buttons `[-][x]` — below this there's no room for legible click targets,
/// and the pair draws all-or-nothing so hit-tests never half-apply.
pub(crate) const BTNS_COLS: u16 = 13;

/// Pixel rect of one 3-cell border button whose leftmost glyph sits at card
/// column `cols - off`. `None` when the card is too narrow for the pair.
pub(crate) fn btn_rect(rect: Rect, cw: f32, ch: f32, off: u16) -> Option<Rect> {
    let (icols, _) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
    let cols = icols + 2;
    if cols < BTNS_COLS {
        return None;
    }
    Some(Rect {
        x: rect.x + f32::from(cols - off) * cw,
        y: rect.y,
        w: 3.0 * cw,
        h: ch,
    })
}

/// The `[x]` close button: the corner slot (card columns `cols-5 ..= cols-3`).
pub(crate) fn close_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 5)
}

/// The `[-]` minimize button, directly left of `[x]` (columns `cols-8 ..= cols-6`).
pub(crate) fn min_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    btn_rect(rect, cw, ch, 8)
}
