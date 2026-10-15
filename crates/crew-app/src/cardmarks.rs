//! The left border's row marks — where a command began, where the shell
//! reported one failed, where an error line is — drawn on the CONTENT's rows.
//!
//! They were glyphs in the card's own cells (`╶`, `▌`), and a card's rows are
//! not its content's: the content sits half the card's sub-cell remainder
//! lower ([`crate::panefit::slack`]), so every mark stood up to half a row
//! above the line it marked. The shell's `error[E0433]` bar straddled the
//! `warning:` line over it by 6 px (glass survey E3). Drawn, a mark lands on
//! its line's own pixels.
use crew_render::{CellView, Paint};

use crate::pane::Pane;
use crate::panecard::Bar;

/// Everything pane `p`'s card draws rather than spells: the readings of
/// [`crate::cardpaint`] and the row marks, on a `cw`×`ch` px cell.
pub fn card_paint(p: &Pane, b: &Bar, cw: f32, ch: f32, now: u64) -> Vec<Paint> {
    let dy = crate::panefit::slack(p.rect, cw, ch).1 / ch;
    // The card's grid: the content grid plus the ring `pane_card` lays out.
    painted(p.grid.cols + 2, p.grid.rows + 2, b, (dy, cw, ch), now)
}

/// Pane `p`'s frame `cells` with the stroke under its drawn readings in
/// their colour ([`crate::cardpaint::tint`]).
pub fn tinted(p: &Pane, mut cells: Vec<CellView>, b: &Bar) -> Vec<CellView> {
    crate::cardpaint::tint(&mut cells, p.grid.cols + 2, p.grid.rows + 2, b);
    cells
}

/// [`card_paint`] for a `cols`×`rows` card whose content starts `dy` rows
/// below its row 1.
pub fn painted(cols: u16, rows: u16, b: &Bar, at: (f32, f32, f32), now: u64) -> Vec<Paint> {
    let (dy, cw, ch) = at;
    // Notches first: the thumb is drawn over the border they ride.
    let mut out = notches(cols, rows, b, cw, ch);
    out.extend(crate::cardpaint::card_paint(cols, rows, b, (cw, ch), now));
    out.extend(marks(rows, b, dy, cw, ch));
    out
}

/// Landmarks (`]` / `[`) and search hits down the right border of a
/// `cols`×`rows` card: one notch per row, placed proportionally, from the
/// stroke into the card. They were `┈` / `━` glyphs in the border cell, a
/// whole cell wide across a stroke drawn down its middle, so half of every
/// tick stood out in the gap between two cards (glass survey E14). A hit
/// outranks a landmark in its row, and the thumb — where you ARE — outranks
/// both, so neither is drawn under it.
pub fn notches(cols: u16, rows: u16, b: &Bar, cw: f32, ch: f32) -> Vec<Paint> {
    if rows < 5 || cols < 2 || b.total == 0 {
        return Vec::new();
    }
    let light = (ch * 0.07).round().max(1.0);
    // The cell's inner half up to the stroke's far side: never past it.
    let w = (((cw - light) / 2.0).floor() + light) / cw;
    let inner = usize::from(rows - 2);
    let thumb = crate::cardpaint::thumb_span(rows, b);
    let mut taken: Vec<u16> = Vec::new();
    let mut out = Vec::new();
    for (lines, fg, weight) in [
        (b.hits, crate::findhl::hit_mark(), 2.0 * light),
        (b.ticks, crew_theme::theme().legend_off, light),
    ] {
        for &line in lines {
            let at = (line.min(b.total - 1) * inner) / b.total;
            let r = 1 + at.min(inner - 1) as u16;
            let mid = f32::from(r) + 0.5;
            if taken.contains(&r) || thumb.is_some_and(|(y, h)| (y..=y + h).contains(&mid)) {
                continue;
            }
            taken.push(r);
            let top = ((ch - weight) / 2.0).floor() / ch;
            let x = f32::from(cols - 1);
            out.push(Paint::solid(x, f32::from(r) + top, w, weight / ch, fg));
        }
    }
    out
}

/// The marks for `b` on a `rows`-row card whose content row 0 starts `dy`
/// rows below the card's row 1, in card cells (`0,0` the top-left corner).
pub fn marks(rows: u16, b: &Bar, dy: f32, cw: f32, ch: f32) -> Vec<Paint> {
    let t = crew_theme::theme();
    // The frame's own rule weight for this cell (`deco::thickness`).
    let light = (ch * 0.07).round().max(1.0);
    // Never into the bottom border's row: past it the stretched frame moves
    // a rect's far edge out by the card's remainder (`stretch::parts`).
    let floor = f32::from(rows.saturating_sub(1));
    let mut out = Vec::new();
    let mut mark = |r: u16, (x, top, w, h): (f32, f32, f32, f32), fg| {
        let y = f32::from(r) + 1.0 + dy + top;
        let h = h.min(floor - y);
        if r + 2 < rows && h > 0.0 {
            out.push(Paint::solid(x, y, w, h, fg));
        }
    };
    // `╶`: from the stroke's centre into the card, across the line's middle.
    // A block the shell reported a failure for wears it heavy, in the alarm
    // colour. An error bar on the same row replaces it: "this failed"
    // outranks "this began".
    let x = ((cw - light) / 2.0).floor() / cw;
    for (rows_of, fg, weight) in [
        (b.cmd_rows, t.legend_off, light),
        (b.fail_rows, t.bell, 2.0 * light),
    ] {
        let top = ((ch - weight) / 2.0).floor() / ch;
        for &r in rows_of.iter().filter(|r| !b.err_rows.contains(r)) {
            mark(r, (x, top, 1.0 - x, weight / ch), fg);
        }
    }
    // `▌`: the cell's left half, the line's whole height.
    for &r in b.err_rows {
        mark(r, (0.0, 0.0, 0.5, 1.0), t.bell);
    }
    out
}

#[cfg(test)]
#[path = "cardmarks_tests.rs"]
mod tests;
