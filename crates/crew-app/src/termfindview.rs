//! Drawing a terminal pane's find bar: every match tinted, the current one
//! stronger, and a one-row bar stamped over the pane's top row (the bottom one when the match is on the top).
//!
//! The pane is cells like everything in crew, so the bar is cells too (the
//! way [`crate::hintlegend`] stamps its sign over the bottom row), not an OS
//! overlay. The top row is the one a find may claim: the prompt and the
//! newest output live at the bottom, and a match is never left under the bar
//! (see [`TermFind::reveal`]).
use crate::pane::{Pane, PaneContent};
use crate::termfind::TermFind;
use crew_render::CellView;

/// How far a match toward the find colour: the others a half measure so the
/// current one — the full colour — is the one the eye lands on.
const OTHER: f32 = 0.45;

/// Tint the pane's find state into `cells`: the open bar's matches and bar,
/// else (scrolled back from a `/find`) the older term wash.
pub(crate) fn wash_pane(
    cells: &mut Vec<CellView>,
    p: &Pane,
    foc: bool,
    scroll: usize,
    last_find: Option<&str>,
) {
    let PaneContent::Terminal(t) = &p.content else {
        return;
    };
    let (cols, rows) = (p.grid.cols, p.grid.rows);
    match &t.find {
        Some(f) => paint(cells, f, t.pty.display_offset(), cols, rows),
        None => match last_find {
            Some(term) if foc && scroll > 0 => {
                crate::findhl::highlight(cells, term, cols, rows);
            }
            _ => {}
        },
    }
}

/// The matches in view tinted, then the bar.
pub(crate) fn paint(cells: &mut Vec<CellView>, f: &TermFind, off: usize, cols: u16, rows: u16) {
    let t = crew_theme::theme();
    let hl = t.find_hl_bg;
    let floor = crew_theme::contrast::text_floor();
    // The current match is the full find colour; on a dark page, where that
    // colour is itself dim, it is also lifted toward the ink so it still
    // stands out from the half-strength rest.
    let now = if t.dark {
        crate::anim::lerp_rgb(hl, t.ink, 0.22)
    } else {
        hl
    };
    let mut level = vec![0u8; cols as usize * rows as usize];
    for r in f.tints(off, cols, rows) {
        let at = r.row as usize * cols as usize;
        for c in r.from..=r.to.min(cols.saturating_sub(1)) {
            let v = &mut level[at + c as usize];
            *v = (*v).max(if r.current { 2 } else { 1 });
        }
    }
    for c in cells.iter_mut() {
        let lv = level.get(c.row as usize * cols as usize + c.col as usize);
        let bg = match lv {
            Some(2) => now,
            Some(1) => crate::anim::lerp_rgb(c.bg, hl, OTHER),
            _ => continue,
        };
        // The ink was floored against the page it sat on; it has to be
        // re-floored against the tint, or the match is the one thing unread.
        c.fg = crew_theme::readable::enforced(c.fg, bg, floor);
        c.bg = bg;
        c.mark = true;
    }
    bar(cells, f, f.bar_row(off, rows), cols, rows);
}

/// The bar over `row`: ` find query▏        3/17 `.
fn bar(cells: &mut Vec<CellView>, f: &TermFind, row: u16, cols: u16, rows: u16) {
    if rows < 2 || cols < 8 {
        return;
    }
    let t = crew_theme::theme();
    let bg = crate::anim::lerp_rgb(t.page_bg, t.ink, 0.10);
    cells.retain(|c| c.row != row);
    for col in 0..cols {
        cells.push(CellView {
            col,
            row,
            c: ' ',
            bg,
            ..Default::default()
        });
    }
    // The bell colour is the theme's own warning: floored at the weaker 3:1 a
    // warning is judged by, so it stays the colour the theme picked.
    let mut put = |col: u16, c: char, fg: (u8, u8, u8), floor: f32| {
        if let Some(cell) = cells
            .iter_mut()
            .rev()
            .find(|x| x.row == row && x.col == col)
        {
            cell.c = c;
            cell.fg = crew_theme::readable::enforced(fg, bg, floor);
        }
    };
    let count = f.count();
    let room = count
        .as_ref()
        .map_or(0, |(s, _)| crate::chatwidth::str_w(s) + 2);
    let left = "find ".chars().map(|c| (c, t.text_muted));
    let query = f.query.chars().map(|c| (c, t.ink));
    let caret = std::iter::once(('\u{258f}', t.ink));
    let limit = cols.saturating_sub(room as u16);
    crate::chatwidth::place_row(1, limit, left.chain(query).chain(caret), |x, c, _, fg| {
        put(x, c, fg, 4.5)
    });
    if let Some((text, bad)) = count {
        let (fg, floor) = if bad {
            (t.bell, 3.0)
        } else {
            (t.text_muted, 4.5)
        };
        let at = cols.saturating_sub(crate::chatwidth::str_w(&text) as u16 + 1);
        crate::chatwidth::place_row(at, cols, text.chars().map(|c| (c, fg)), |x, c, _, fg| {
            put(x, c, fg, floor)
        });
    }
}

#[cfg(test)]
#[path = "termfindview_tests.rs"]
mod tests;
