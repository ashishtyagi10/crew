//! The collapsed nav: the activity bar's icons on top, the pane numbers the rail always carried
//! under a rule, the rail's readings at the foot, and the settings gear on the last row.
//! Split from [`crate::navrail`] for the line cap; the bar's rows are [`crate::navbar`]'s.
use crew_render::{CellView, Paint};

use crate::navbar::{Badges, BarHit, HEAD_ROWS};

/// What the collapsed card is drawn from this frame.
pub(crate) struct RailBody<'a> {
    pub list: &'a crate::tabnav::NavList,
    pub foot: &'a crate::navrailfoot::Foot,
    pub hover: Option<BarHit>,
    pub badges: Badges,
    pub spin: char,
}

/// Row the first pane number stands on: under the icons and their rule.
pub(crate) const LIST_TOP: u16 = HEAD_ROWS + 1;

/// The rows of a collapsed card that are on screen. It is drawn level with the panes
/// (`panellevel`), which drops its interior by up to a row and clips the last one — where the
/// gear stands. The draw and both hit paths read this, so the gear is never drawn off the card
/// nor answers a click where it is not.
pub(crate) fn visible_rows(rows: u16, dy: f32) -> u16 {
    match dy > 0.0 {
        true => rows.saturating_sub(1),
        false => rows,
    }
}

/// Rows above the gear and the air over it — where the list and the readings must end. The
/// gear is drawn taller than its row (`navicons`), so the row above it is its too.
pub(crate) fn foot_rows(rows: u16) -> u16 {
    crate::navbar::row_of(BarHit::Settings, rows).map_or(rows, |r| r.saturating_sub(1))
}

/// Rows the pane list may take — what the draw gives it, and what a click maps back over.
pub(crate) fn list_rows(rows: u16) -> u16 {
    foot_rows(rows).saturating_sub(LIST_TOP)
}

/// The collapsed card's cells and paint, in its interior's coordinates.
pub(crate) fn body(b: &RailBody, cols: u16, rows: u16, aspect: f32) -> (Vec<CellView>, Vec<Paint>) {
    let t = crew_theme::theme();
    let mut cells = crate::navbar::bar_cells(None, b.badges, rows);
    // The rule under the icons: what is above it is where to go, what is below it is what is open.
    for col in 0..cols {
        cells.push(CellView {
            col,
            row: HEAD_ROWS,
            c: '\u{2500}',
            fg: t.border_normal,
            bg: t.page_bg,
            ..Default::default()
        });
    }
    // The gear holds the foot of the bar; the list and the readings share what is between.
    let room = list_rows(rows);
    for mut c in b.list.rail_cells(cols, room, b.spin) {
        c.row += LIST_TOP;
        cells.push(c);
    }
    let used = LIST_TOP + b.list.rail_len().min(usize::from(room)) as u16;
    let (foot, mut paint) = crate::navrailfoot::foot(b.foot, cols, foot_rows(rows), used, aspect);
    cells.extend(foot);
    paint.extend(crate::navicons::bar_paint(None, b.hover, rows, aspect));
    (cells, paint)
}

/// One row per pane: caret, number, mark. `spin` is the busy frame the full
/// list uses, so a pane that is working turns the same way in both.
pub(crate) fn rail_cells(
    panes: &[crate::panelist::PaneRow],
    cols: u16,
    rows: u16,
    spin: char,
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    for (row, p) in panes.iter().take(usize::from(rows)).enumerate() {
        let row = row as u16;
        let head = format!("{}{}", if p.focused { '\u{25b8}' } else { ' ' }, p.index);
        let head_fg = match p.focused || p.hovered {
            true => crate::accentink::accent_ink(),
            false => t.text_muted,
        };
        // Two columns of air held back for the mark: the number is what gets
        // cut if a crew ever runs to four digits, never the thing that says
        // the pane needs you.
        let budget = cols.saturating_sub(2);
        crate::navtext::put_at(&mut out, &head, 0, row, budget, head_fg);
        // The mark keeps the full list's priority and colours: a raised
        // attention wins, then work in progress, then lines not yet read —
        // then the full list's `+` for a pane off the grid, which the rail
        // did not carry, so a minimized pane was a bare number here.
        let mark = match (p.attention, p.busy, p.activity || p.unread > 0) {
            (Some((glyph, on)), _, _) => on.then_some((glyph, t.bell)),
            (None, true, _) => Some((spin, crate::accentink::accent_ink())),
            (None, false, true) => Some(('\u{25cf}', t.activity)),
            _ => p.minimized.then_some(('+', crate::accentink::accent_ink())),
        };
        if let Some((glyph, fg)) = mark {
            let at = cols.saturating_sub(1);
            crate::navtext::put_at(&mut out, &glyph.to_string(), at, row, cols, fg);
        }
    }
    out
}

#[cfg(test)]
#[path = "navrailbar_tests.rs"]
mod tests;
