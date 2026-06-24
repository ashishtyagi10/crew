//! Fieldset card drawing: the rounded border + legend that frames every panel
//! (panes via [`pane_card`], the sidebar/welcome via [`push_card`]). No title
//! bars — a panel is just a border with a legend on its top edge, so the UI
//! reads as boxes drawn on one canvas.
use crew_render::{CellView, PaneScene};

use crate::boxdraw::titled_card;
use crate::layout::Rect;

pub(crate) const ACCENT: (u8, u8, u8) = (0, 255, 160);
pub(crate) const SCROLL_HINT: (u8, u8, u8) = (230, 180, 90);
pub(crate) const ACTIVITY: (u8, u8, u8) = (120, 200, 255);
pub(crate) const BELL: (u8, u8, u8) = (240, 210, 90);
pub(crate) const BROADCAST: (u8, u8, u8) = (220, 120, 200);
const BORDER_ON: (u8, u8, u8) = (210, 210, 220);
const BORDER_OFF: (u8, u8, u8) = (110, 110, 120);
const LEGEND_OFF: (u8, u8, u8) = (140, 140, 150);
const CANVAS_BG: (u8, u8, u8) = (0, 0, 0);

/// Inputs for one pane's fieldset border.
pub(crate) struct Bar<'a> {
    pub index: Option<usize>,
    pub title: &'a str,
    pub focused: bool,
    /// Lines scrolled back from the live bottom (0 = at the bottom).
    pub scroll: usize,
    pub activity: bool,
    pub bell: bool,
    /// This pane is receiving broadcast (synchronized) input.
    pub broadcast: bool,
}

/// Overwrite (or append) the cell at `(col, row)` in `v` — used to drop status
/// glyphs onto the already-drawn top border.
fn put(v: &mut Vec<CellView>, col: u16, row: u16, c: char, fg: (u8, u8, u8)) {
    if let Some(cell) = v.iter_mut().find(|x| x.col == col && x.row == row) {
        (cell.c, cell.fg, cell.bg) = (c, fg, CANVAS_BG);
    } else {
        v.push(CellView {
            col,
            row,
            c,
            fg,
            bg: CANVAS_BG,
            bold: false,
            italic: false,
        });
    }
}

/// Build the fieldset border for a pane with a `gcols × grows` interior: a
/// rounded card whose top border carries the legend (left) and right-aligned
/// status glyphs. No filled title bar — just the frame on the canvas.
pub(crate) fn pane_card(gcols: u16, grows: u16, b: &Bar) -> Vec<CellView> {
    let (cols, rows) = (gcols + 2, grows + 2);
    let (border, legend) = if b.focused {
        (BORDER_ON, ACCENT)
    } else {
        (BORDER_OFF, LEGEND_OFF)
    };
    let label = match b.index {
        Some(n) => format!("{n} {}", b.title),
        None => b.title.to_string(),
    };
    let mut v = titled_card(cols, rows, &label, border, legend, CANVAS_BG);
    if v.is_empty() {
        return v;
    }
    // Status glyphs ride the top-right border, stepping left from the corner.
    let mut rx = cols.saturating_sub(3);
    if b.scroll > 0 {
        let s = format!("⇡{}", b.scroll);
        let w = s.chars().count() as u16;
        if rx + 1 > w {
            let start = rx + 1 - w;
            for (i, ch) in s.chars().enumerate() {
                put(&mut v, start + i as u16, 0, ch, SCROLL_HINT);
            }
            rx = start.saturating_sub(2);
        }
    }
    for (on, c, fg) in [
        (b.broadcast, '»', BROADCAST),
        (b.activity, '●', ACTIVITY),
        (b.bell, '!', BELL),
    ] {
        if on && rx > 1 {
            put(&mut v, rx, 0, c, fg);
            rx = rx.saturating_sub(2);
        }
    }
    v
}

/// Push a fieldset card for a non-pane panel (sidebar, welcome) into `scenes`:
/// an inset content buffer plus a dim border card carrying `legend`. `content`
/// builds the interior cells at the inset `(cols, rows)` grid. Content and
/// border ride separate buffers, like panes, so the border never shifts content.
pub fn push_card(
    scenes: &mut Vec<PaneScene>,
    rect: Rect,
    cw: f32,
    ch: f32,
    legend: &str,
    content: impl FnOnce(u16, u16) -> Vec<CellView>,
) {
    let icols = ((rect.w / cw).floor() as u16).saturating_sub(2).max(1);
    let irows = ((rect.h / ch).floor() as u16).saturating_sub(2).max(1);
    scenes.push(PaneScene {
        cells: content(icols, irows),
        x: rect.x + cw,
        y: rect.y + ch,
        w: (rect.w - 2.0 * cw).max(0.0),
        h: (rect.h - 2.0 * ch).max(0.0),
        focused: false,
        bordered: false,
        overlay: false,
    });
    scenes.push(PaneScene {
        cells: titled_card(
            icols + 2,
            irows + 2,
            legend,
            BORDER_OFF,
            LEGEND_OFF,
            CANVAS_BG,
        ),
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        focused: false,
        bordered: false,
        overlay: false,
    });
}

#[cfg(test)]
#[path = "paneview_tests.rs"]
mod tests;
