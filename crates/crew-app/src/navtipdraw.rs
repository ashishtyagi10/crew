//! The nav's hover card (`navtip`) as cells and a scene: a fieldset card like a toast's —
//! the section's name on its top border, the whole text inside, wrapped — floating just right
//! of the nav with its first line level with the row it is for.
use crew_render::{CellView, PaneScene};

use crate::app::CrewApp;
use crate::navsection::NavSection;

/// The widest the card's text runs: enough for a deep path in two or three lines, narrow
/// enough to read as a note beside the row rather than a panel.
pub(crate) const MAX_TEXT_COLS: usize = 56;
/// Text rows the card may take; a longer text is cut on the last.
pub(crate) const MAX_LINES: usize = 4;
/// The border, a column of air, the text, a column of air, the border.
const PAD: usize = 4;

/// `whole`'s paragraphs wrapped to `max` columns, at most [`MAX_LINES`] of them; the last
/// row marks the cut when there is more.
pub(crate) fn lines(whole: &[String], max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut out: Vec<String> = whole.iter().flat_map(|p| wrap(p, max)).collect();
    if out.len() > MAX_LINES {
        let rest = out[MAX_LINES - 1..].concat();
        out.truncate(MAX_LINES);
        out[MAX_LINES - 1] = crate::chatwidth::clip_w(&rest, max);
    }
    out
}

/// `p` in rows of `max` columns, broken after a space or a path's separator — a path wraps
/// at its folders (`…/2026/` then `quarterly-reports/drafts`), not mid-name, as prose wraps
/// at its words. Only a run with neither, wider than a row, is cut where the row ends.
fn wrap(p: &str, max: usize) -> Vec<String> {
    let w = crate::chatwidth::str_w;
    let mut out = vec![String::new()];
    for tok in p.split_inclusive([' ', '/', '\\']) {
        let line = out.last_mut().expect("a row");
        if !line.is_empty() && w(line) + w(tok.trim_end()) > max {
            out.push(String::new());
        }
        for c in tok.chars() {
            let line = out.last_mut().expect("a row");
            if !line.is_empty() && w(line) + w(c.encode_utf8(&mut [0; 4])) > max {
                if c == ' ' {
                    continue; // the break itself
                }
                out.push(String::new());
            }
            out.last_mut().expect("a row").push(c);
        }
    }
    out.iter()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The card's cells for `lines` under `legend`, and its size in cells.
pub(crate) fn cells(legend: &str, lines: &[String]) -> (Vec<CellView>, u16, u16) {
    let t = crew_theme::theme();
    let widest = lines.iter().map(|l| crate::chatwidth::str_w(l)).max();
    let legend_w = crate::chatwidth::str_w(legend) + 6;
    let cols = (widest.unwrap_or(0) + PAD).max(legend_w) as u16;
    let rows = lines.len() as u16 + 2;
    let mut out =
        crate::boxdraw::titled_card(cols, rows, legend, t.border_normal, t.legend_off, t.page_bg);
    crate::modernring::quiet(&mut out, cols, rows, t.border_normal);
    for (i, line) in lines.iter().enumerate() {
        let row = 1 + i as u16;
        crate::chatwidth::place_row(
            2,
            cols - 2,
            line.chars().map(|c| (c, t.ink)),
            |col, c, marks, fg| {
                out.push(CellView {
                    col,
                    row,
                    c,
                    marks,
                    fg,
                    bg: t.page_bg,
                    ..Default::default()
                })
            },
        );
    }
    (out, cols, rows)
}

/// The card's legend: what its text is, since the section is already named beside it (a Far
/// row's card said `far` beside the Far pane's own `far`).
fn legend(section: NavSection) -> &'static str {
    match section {
        NavSection::Far => "path",
        NavSection::Inbox => "thread",
        NavSection::Panes => "pane",
        _ => "name",
    }
}

impl CrewApp {
    /// The card for `whole`, a row of `section`, standing right of the nav at the pointer's
    /// row — `None` before the renderer has reported geometry, or with no room right of it.
    pub(crate) fn nav_tip_scene(&self, section: NavSection, whole: &[String]) -> Option<PaneScene> {
        let (cw, ch, sw, sh, _) = self.frame_geometry()?;
        let (panel, ..) = self.nav_hit_geometry()?;
        let x = panel.x + panel.w + self.gutter().x;
        let room = ((sw - x) / cw).floor() as usize;
        let max = MAX_TEXT_COLS.min(room.checked_sub(PAD + 1)?);
        let lines = lines(whole, max);
        if lines.is_empty() {
            return None;
        }
        let (cells, cols, rows) = cells(legend(section), &lines);
        let h = f32::from(rows) * ch;
        // The pointer's row, top edge; the card's first line on it, its border a row above.
        let row_top = panel.y + ((self.cursor.1 - panel.y) / ch).floor() * ch;
        let y = (row_top - ch).min(sh - h).max(0.0);
        Some(PaneScene {
            cells,
            x,
            y,
            w: f32::from(cols) * cw,
            h,
            focused: false,
            bordered: false,
            // Exactly its frame, so it casts its own shadow, as the pop-ups and toasts do.
            glass: true,
            scan: -1.0,
            lift: crate::popupplace::FLOAT,
            glint: -1.0,
            pictures: Vec::new(),
            stretch: false,
            overlay: true,
            scroll_px: 0.0,
            paint: Vec::new(),
        })
    }
}
