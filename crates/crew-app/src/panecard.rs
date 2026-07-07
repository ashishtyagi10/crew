//! Fieldset card drawing: the rounded border + legend that frames every panel
//! (panes via [`pane_card`], the sidebar/welcome via [`push_card`]). No title
//! bars — a panel is just a border with a legend on its top edge, so the UI
//! reads as boxes drawn on one canvas.
use crew_render::{CellView, PaneScene};

use crate::boxdraw::titled_card;
use crate::layout::Rect;

pub(crate) use crate::palette::accent;

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
    /// `Some(now_ms)` when the pane is busy: animate an indeterminate sweep along
    /// the bottom border at that time. `None` leaves the border static.
    pub busy: Option<u64>,
    /// Draw the `▾` minimize button on the top border (full grid tiles only —
    /// not the zoomed view or strip thumbnails). Click regions come from
    /// [`min_btn_rect`], which shares [`MIN_BTN_COLS`] so draw and hit agree.
    pub min_btn: bool,
}

/// Narrowest card (in cells, border included) that carries the minimize
/// button — below this there's no room for a legible click target.
const MIN_BTN_COLS: u16 = 8;

/// Pixel rect of the `▾` minimize button on a full tile at `rect`: a
/// 3-cell-wide row-0 target centered on the glyph (card column `cols - 3`).
/// `None` when the card is too narrow to carry the button. Mirrors
/// `relayout_one`'s rect→cols math so it lands on the drawn glyph exactly.
pub(crate) fn min_btn_rect(rect: Rect, cw: f32, ch: f32) -> Option<Rect> {
    let icols = ((rect.w / cw).floor() as u16).saturating_sub(2).max(1);
    let cols = icols + 2;
    if cols < MIN_BTN_COLS {
        return None;
    }
    let glyph = cols - 3;
    Some(Rect {
        x: rect.x + f32::from(glyph - 1) * cw,
        y: rect.y,
        w: 3.0 * cw,
        h: ch,
    })
}

/// Overwrite (or append) the cell at `(col, row)` in `v` — used to drop status
/// glyphs onto the already-drawn top border.
fn put(v: &mut Vec<CellView>, col: u16, row: u16, c: char, fg: (u8, u8, u8)) {
    if let Some(cell) = v.iter_mut().find(|x| x.col == col && x.row == row) {
        (cell.c, cell.fg, cell.bg) = (c, fg, crew_theme::theme().page_bg);
    } else {
        v.push(CellView {
            col,
            row,
            c,
            fg,
            bg: crew_theme::theme().page_bg,
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
        (crew_theme::theme().border_focused, accent())
    } else {
        (
            crew_theme::theme().border_normal,
            crew_theme::theme().legend_off,
        )
    };
    let label = match b.index {
        Some(n) => format!("{n} {}", b.title),
        None => b.title.to_string(),
    };
    let mut v = titled_card(
        cols,
        rows,
        &label,
        border,
        legend,
        crew_theme::theme().page_bg,
    );
    if v.is_empty() {
        return v;
    }
    // The focused card's legend goes bold: the active tile reads at a glance
    // without any extra chrome on the canvas.
    if b.focused {
        for cell in v.iter_mut().filter(|c| c.row == 0 && c.fg == legend) {
            cell.bold = true;
        }
    }
    // Status glyphs ride the top-right border, stepping left from the corner.
    let mut rx = cols.saturating_sub(3);
    // The minimize button claims the corner slot; status glyphs step past it.
    if b.min_btn && cols >= MIN_BTN_COLS {
        put(&mut v, rx, 0, '▾', legend);
        rx = rx.saturating_sub(2);
    }
    if b.scroll > 0 {
        let s = format!("⇡{}", b.scroll);
        let w = s.chars().count() as u16;
        if rx + 1 > w {
            let start = rx + 1 - w;
            for (i, ch) in s.chars().enumerate() {
                put(
                    &mut v,
                    start + i as u16,
                    0,
                    ch,
                    crew_theme::theme().status_fg,
                );
            }
            rx = start.saturating_sub(2);
        }
    }
    for (on, c, fg) in [
        (b.broadcast, '»', crew_theme::theme().broadcast),
        (b.activity, '●', crew_theme::theme().activity),
        (b.bell, '!', crew_theme::theme().bell),
    ] {
        if on && rx > 1 {
            put(&mut v, rx, 0, c, fg);
            rx = rx.saturating_sub(2);
        }
    }
    // Indeterminate progress: a sweep along the bottom border while busy.
    if let Some(now) = b.busy {
        let bottom = rows - 1;
        for (col, fg) in crate::progress::sweep(cols, now) {
            put(&mut v, col, bottom, '━', fg);
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
            crew_theme::theme().border_normal,
            crew_theme::theme().legend_off,
            crew_theme::theme().page_bg,
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
