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
    /// `Some(now_ms)` when the pane is busy: animate an in-pane indeterminate
    /// rain patch (bottom-right corner) at that time. `None` leaves it off.
    pub busy: Option<u64>,
    /// Draw the `[-][x]` minimize and close buttons on the top border (full grid
    /// tiles and the zoomed tile — not strip thumbnails). Click regions come from
    /// [`min_btn_rect`] and [`close_btn_rect`], which both share [`BTNS_COLS`]
    /// so draw and hit agree.
    pub min_btn: bool,
}

/// Narrowest card (in cells, border included) that carries the border
/// buttons `[-][x]` — below this there's no room for legible click targets,
/// and the pair draws all-or-nothing so hit-tests never half-apply.
const BTNS_COLS: u16 = 13;

/// Pixel rect of one 3-cell border button whose leftmost glyph sits at card
/// column `cols - off`. `None` when the card is too narrow for the pair.
fn btn_rect(rect: Rect, cw: f32, ch: f32, off: u16) -> Option<Rect> {
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
    // Each pane carries a signature hue derived from its title — the same hash
    // the crew roster uses for agent names, so a swarm agent's pane and its
    // roster row read as one colour. Focus stays legible via bold + the
    // focused border; the unfocused legend recedes toward `legend_off`.
    let hue = crate::chatroster::agent_color(b.title);
    let (border, legend) = if b.focused {
        (crew_theme::theme().border_focused, hue)
    } else {
        (
            crew_theme::theme().border_normal,
            crate::anim::lerp_rgb(hue, crew_theme::theme().legend_off, 0.55),
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
    // The [-][x] buttons claim the corner slots; status glyphs step past them.
    if b.min_btn && cols >= BTNS_COLS {
        for (i, ch) in "[-][x]".chars().enumerate() {
            put(&mut v, cols - 8 + i as u16, 0, ch, legend);
        }
        rx = cols.saturating_sub(10);
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
    // Indeterminate progress: a small "matrix rain" patch inset in the
    // bottom-right interior corner while busy — in-pane, not a border sweep.
    if let Some(now) = b.busy {
        overlay_rain(&mut v, cols, rows, now);
    }
    v
}

/// Overlay the busy rain indicator: a compact [`crate::charrain`] region in the
/// bottom-right interior corner (right/bottom-aligned, one cell clear of the
/// border). Cells replace whatever the border buffer held there, so the drops
/// win cleanly. Skipped when the card is too small to host a legible patch.
fn overlay_rain(v: &mut Vec<CellView>, cols: u16, rows: u16, now: u64) {
    let w = cols.saturating_sub(2).min(10);
    let h = rows.saturating_sub(2).min(3);
    if w < 3 || h < 2 {
        return;
    }
    let (left, top) = (cols - 1 - w, rows - 1 - h);
    let (head, trail, bg) = (accent(), (40, 40, 48), crew_theme::theme().page_bg);
    let mut drops = Vec::new();
    // now_ms → a calm ~few-cells/second fall (charrain scales this down again).
    crate::charrain::rain(&mut drops, top, left, w, h, now / 90, head, trail, bg);
    for d in drops {
        v.retain(|c| !(c.col == d.col && c.row == d.row));
        v.push(d);
    }
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
    let (icols, irows) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);
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
