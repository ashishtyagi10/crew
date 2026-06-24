//! Rendering panes to `PaneScene`s. Each pane is a fieldset card — a rounded
//! border whose top edge carries the pane's name/title as a legend (no title
//! bar) and status glyphs (scrollback, activity, bell, broadcast) on the
//! top-right border. Content is drawn in the interior, inset past the border.
use crew_render::{CellView, PaneScene};

use crate::boxdraw::titled_card;
use crate::pane::{Pane, PaneContent};

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const SCROLL_HINT: (u8, u8, u8) = (230, 180, 90);
const ACTIVITY: (u8, u8, u8) = (120, 200, 255);
const BELL: (u8, u8, u8) = (240, 210, 90);
const BROADCAST: (u8, u8, u8) = (220, 120, 200);
const BORDER_ON: (u8, u8, u8) = (210, 210, 220);
const BORDER_OFF: (u8, u8, u8) = (110, 110, 120);
const LEGEND_OFF: (u8, u8, u8) = (140, 140, 150);
const CANVAS_BG: (u8, u8, u8) = (0, 0, 0);

/// Inputs for one pane's fieldset border.
struct Bar<'a> {
    index: Option<usize>,
    title: &'a str,
    focused: bool,
    /// Lines scrolled back from the live bottom (0 = at the bottom).
    scroll: usize,
    activity: bool,
    bell: bool,
    /// This pane is receiving broadcast (synchronized) input.
    broadcast: bool,
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
fn pane_card(gcols: u16, grows: u16, b: &Bar) -> Vec<CellView> {
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

/// Build the `PaneScene`s for one frame (for `renderer.frame`). Each pane yields
/// **two** scenes — the content, inset by one cell on every side, and the border
/// card around it — kept in separate text buffers so the box-drawing border
/// glyphs never share a line with (and so never shift) the content. `broadcast`
/// marks terminal panes receiving synchronized input; `find` is the active
/// `/find` term, highlighted in the focused pane while scrolled back.
pub fn build_scenes(
    panes: &[Pane],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
    cw: f32,
    ch: f32,
) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    let mut scenes = Vec::with_capacity(panes.len() * 2);
    for (i, p) in panes.iter().enumerate() {
        let foc = focused == Some(i);
        let mut cells = p.cells(foc);
        let is_term = matches!(&p.content, PaneContent::Terminal(_));
        let scroll = match &p.content {
            PaneContent::Terminal(t) => t.pty.display_offset(),
            _ => 0,
        };
        // Tint http(s) URLs blue so they read as clickable (Cmd+click opens).
        if is_term {
            crate::linkhl::colorize(&mut cells, p.grid.cols, p.grid.rows);
        }
        // Wash search matches in the focused terminal while viewing a /find
        // result (scrolled back); it self-clears on return to the bottom.
        if foc && is_term && scroll > 0 {
            if let Some(term) = find {
                crate::findhl::highlight(&mut cells, term, p.grid.cols, p.grid.rows);
            }
        }
        let r = p.rect;
        // Content: its own buffer, inset one cell past the top-left border so it
        // starts exactly on the grid (no leading border glyph to push it).
        scenes.push(PaneScene {
            cells,
            x: r.x + cw,
            y: r.y + ch,
            w: (r.w - 2.0 * cw).max(0.0),
            h: (r.h - 2.0 * ch).max(0.0),
            focused: foc,
            bordered: false,
            overlay: false,
        });
        // Border card: the rounded frame + legend + status, drawn over the rect.
        let title = p.title_text();
        scenes.push(PaneScene {
            cells: pane_card(
                p.grid.cols,
                p.grid.rows,
                &Bar {
                    index: multi.then_some(i + 1),
                    title: &title,
                    focused: foc,
                    scroll,
                    activity: p.activity && !foc,
                    bell: p.bell && !foc,
                    broadcast: broadcast && is_term,
                },
            ),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            focused: foc,
            bordered: false,
            overlay: false,
        });
    }
    scenes
}

#[cfg(test)]
#[path = "paneview_tests.rs"]
mod tests;
