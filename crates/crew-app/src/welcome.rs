//! The empty-screen welcome: a bounded "matrix rain" glyph field centred on
//! the canvas, with a tagline + keyboard hint below it and a version stamp in
//! the corner. (Replaced the rotating ASCII globe — see [`crate::charrain`].)
use crew_render::CellView;

use crate::charrain::{rain, RAIN_H, RAIN_MIN_H, RAIN_MIN_W, RAIN_W};
use crate::welcomeart::{frame, nameplate, push_hint, push_spans, push_str};
use crate::welcometext::{chord_fg, fits, hint_spans, restore_hint, whats_new, TAGLINE};

/// Poll ticks per rendered frame. The tick doubles as the rain's clock, so this
/// sets the fall speed as well as the frame rate: at the loop's ~62 Hz this
/// lands the welcome field on the same calm few-cells-per-second cadence as the
/// busy patch in `panecard`. Rain moves in whole cells, so ~10 fps still
/// oversamples the fastest column.
pub const ANIM_DIV: u64 = 6;

/// Width-to-height ratio of the rain box (4:1 cells — a wide, low rectangle
/// at the terminal's ~2:1 cell aspect) — derives `h` from `w` without
/// hardcoding the divisor.
const ASPECT: u16 = RAIN_W / RAIN_H;

// Compile-time guard: RAIN_MIN_H must keep tracking RAIN_MIN_W's aspect ratio,
// so this file's `ASPECT`-based derivation never silently drifts from
// charrain.rs's floor.
const _: () = assert!(
    RAIN_MIN_H == RAIN_MIN_W / ASPECT,
    "RAIN_MIN_H must track RAIN_MIN_W's aspect"
);

/// Whether this poll `tick` should redraw the welcome screen.
pub fn anim_should_redraw(tick: u64) -> bool {
    tick.is_multiple_of(ANIM_DIV)
}

/// Largest even rain-box width `w` (rendered at height `w/2`) such that the
/// box + blank row + tagline + hint stack (`h + 3` rows) centres within
/// `rows`, and `w` (plus a 2-col margin) fits within `cols` — capped at
/// `charrain::RAIN_W`, floored at `charrain::RAIN_MIN_W`. `None` when nothing
/// fits — the caller falls back to the single-line banner.
fn rain_width(cols: u16, rows: u16) -> Option<u16> {
    let max_w = cols.saturating_sub(2).min(RAIN_W);
    let mut w = max_w - max_w % 2;
    while w >= RAIN_MIN_W {
        if w / ASPECT + 3 < rows {
            return Some(w);
        }
        w -= 2;
    }
    None
}

/// Render one animation frame: the rain field centred, tagline + hint below
/// it (plus a `/restore` hint when a session snapshot exists), version stamp
/// bottom-right. Falls back to a spaced single-line "CREW" when nothing
/// rain-sized fits. All cells stay within `cols × rows`.
// rustfmt::skip preserves compact inline struct literals.
#[rustfmt::skip]
pub fn welcome_cells_animated(
    cols: u16,
    rows: u16,
    tick: u64,
    restore: Option<usize>,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let bg = t.page_bg;

    if let Some(w) = rain_width(cols, rows) {
        let h = w / ASPECT;
        let top = (rows - (h + 3)) / 2;
        let left = (cols - w) / 2;
        // The rain falls INSIDE the frame (the box's outer ring), and the
        // CREW nameplate sits over its centre — glyphs stream around it.
        rain(
            &mut cells,
            top + 1,
            left + 1,
            w - 2,
            h - 2,
            tick,
            t.ink,
            t.text_muted,
            bg,
        );
        frame(&mut cells, top, left, w, h, t.text_muted, bg);
        nameplate(&mut cells, top, left, w, h, t.ink, bg);

        let tl_row = top + h + 1;
        let tl_w = TAGLINE.chars().count() as u16;
        if tl_row < rows && fits(tl_w as usize, cols) {
            push_str(
                &mut cells,
                tl_row,
                (cols - tl_w) / 2,
                TAGLINE,
                t.hint_fg,
                bg,
            );
        }
        let hint_row = tl_row + 1;
        if hint_row < rows {
            push_hint(&mut cells, hint_row, cols, t.hint_fg, bg);
        }
        // What this build brought. Crew ships often and every release's
        // headline is compiled in already; a first frame that says what is
        // new is how any of it gets found.
        let news_row = hint_row + 1;
        if news_row + 1 < rows {
            if let Some(line) = whats_new(usize::from(cols)) {
                let w = line.chars().count() as u16;
                // `dim`, like the version stamp: this is meta about the
                // build rather than part of the welcome itself — and the
                // rain is told apart from the text below it by colour.
                push_str(&mut cells, news_row, (cols - w) / 2, &line, t.dim, bg);
            }
        }
        if let Some(n) = restore {
            let line = restore_hint(n);
            let (row, w) = (hint_row + 3, line.chars().count() as u16);
            // `row + 1 < rows`: the bottom row belongs to the version stamp
            // (drawn after, last-write-wins) — skip rather than collide.
            if row + 1 < rows && fits(w as usize, cols) {
                // `/restore` is a thing to type, so it wears the accent the
                // opening hint's chords do.
                let spans = hint_spans(&line, chord_fg(), t.hint_fg);
                push_spans(&mut cells, row, (cols - w) / 2, &spans, bg);
            }
        }
    } else {
        // Fallback: spaced single-line "CREW" — same layout math as the old
        // figlet-era fallback, minus the deleted per-column shimmer (static ink).
        let letters: Vec<char> = "CREW".chars().collect();
        let span = (letters.len() as u16 - 1) * 2 + 1;
        if span < cols {
            let row = rows / 2;
            let start = (cols - span) / 2;
            for (i, &ch) in letters.iter().enumerate() {
                cells.push(CellView {
                    col: start + i as u16 * 2,
                    row,
                    c: ch,
                    fg: t.ink,
                    bg,
                    bold: true,
                    italic: false,
                    ..Default::default()
                });
            }
            let hint_row = row + 2;
            if hint_row < rows {
                push_hint(&mut cells, hint_row, cols, t.hint_fg, bg);
            }
        }
    }

    // Version stamp bottom-right.
    let ver = concat!("v", env!("CARGO_PKG_VERSION"));
    let vw = ver.chars().count() as u16;
    if vw + 1 < cols {
        push_str(&mut cells, rows - 1, cols - vw - 1, ver, t.dim, bg);
    }
    cells
}

#[cfg(test)]
#[path = "welcome_tests.rs"]
mod tests;
