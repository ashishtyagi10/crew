//! The WAITING ON YOU card: the panes that need a human — a terminal stopped
//! at a prompt, a plan awaiting yes or no — and the tasks in flight. Each
//! row is a place to go; clicking one focuses its pane (`hit`).
use crew_render::CellView;

use crate::navglance::{Wait, WaitRow};
use crate::palette::accent;

/// Most rows the card shows, however tall the nav: past this it is a list,
/// and the pane list below is the list.
pub const WAIT_MAX: usize = 6;

const TEXT_COL: u16 = 2;

/// Render the card: the rule (with the count when something is waiting),
/// then up to `max_lines` rows. Empty when there is no room.
pub(crate) fn waiting_cells(rows: &[WaitRow], cols: u16, max_lines: usize) -> Vec<CellView> {
    if rows.is_empty() || max_lines == 0 || cols < 8 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let real = rows.iter().filter(|r| r.kind != Wait::Quiet).count();
    let key = if real > 0 {
        real.to_string()
    } else {
        String::new()
    };
    // The full title on a nav wide enough to say it; never a clipped one.
    let title = if cols >= 24 {
        "WAITING ON YOU"
    } else {
        "WAITING"
    };
    let mut out = crate::boxdraw::section_header_key(
        title,
        &key,
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    );
    let max_col = cols.saturating_sub(1);
    let room = usize::from(max_col.saturating_sub(TEXT_COL));
    for (k, r) in rows.iter().take(max_lines).enumerate() {
        let fg = match r.kind {
            Wait::Blocked => t.bell,
            Wait::Plan => t.status_fg,
            Wait::Running => accent(),
            Wait::Quiet => t.text_muted,
        };
        let body = crate::chatwidth::clip_w(&r.text, room);
        crate::chatwidth::place_row(
            TEXT_COL,
            max_col,
            body.chars().map(|c| (c, fg)),
            |x, c, fg| {
                out.push(CellView {
                    col: x,
                    row: 1 + k as u16,
                    c,
                    fg,
                    bg: t.page_bg,
                    ..Default::default()
                });
            },
        );
    }
    out
}

#[cfg(test)]
#[path = "navwaiting_tests.rs"]
mod tests;
