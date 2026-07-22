//! The empty-screen welcome: a bounded "matrix rain" glyph field centred on
//! the canvas, with a tagline + keyboard hint below it and a version stamp in
//! the corner. (Replaced the rotating ASCII globe — see [`crate::charrain`].)
use crew_render::CellView;

use crate::charrain::{rain, RAIN_H, RAIN_MIN_H, RAIN_MIN_W, RAIN_W};

const TAGLINE: &str = "fast terminals. clean flow.";
const HINT: &str = "Cmd+T  new shell    ·    /  commands";
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

/// Push every character of `s` as cells starting at `(col, row)`.
// rustfmt::skip keeps the CellView struct literal on one line.
#[rustfmt::skip]
fn push_str(cells: &mut Vec<CellView>, row: u16, col: u16, s: &str, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    for (i, ch) in s.chars().enumerate() {
        cells.push(CellView { col: col + i as u16, row, c: ch, fg, bg, bold: false, italic: false });
    }
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

/// The rectangular frame on the rain box's outer ring: a muted single-line
/// border, so the rain reads as a bounded field rather than loose glyphs.
#[rustfmt::skip]
fn frame(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    if w < 2 || h < 2 { return; }
    let (bot, right) = (top + h - 1, left + w - 1);
    let mut put = |row: u16, col: u16, c: char| {
        cells.push(CellView { col, row, c, fg, bg, bold: false, italic: false });
    };
    for c in left + 1..right {
        put(top, c, '\u{2500}');
        put(bot, c, '\u{2500}');
    }
    for r in top + 1..bot {
        put(r, left, '\u{2502}');
        put(r, right, '\u{2502}');
    }
    put(top, left, '\u{250c}');
    put(top, right, '\u{2510}');
    put(bot, left, '\u{2514}');
    put(bot, right, '\u{2518}');
}

/// The internal `C R E W` nameplate centred in the rain box — the same
/// double-line box the smith splash wears. Every cell (borders, padding,
/// letters) is pushed, so the plate occludes the rain behind it
/// (crew-render's last-write-wins merge) and the glyphs fall AROUND it.
/// Skipped when the box hasn't the room to hold it with a rain margin.
#[rustfmt::skip]
fn nameplate(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, ink: (u8,u8,u8), bg: (u8,u8,u8)) {
    const PLATE: &str = "C R E W";
    const PAD: u16 = 3;
    let inner = PLATE.len() as u16 + PAD * 2;
    let (bw, bh) = (inner + 2, 3u16);
    if w < bw + 4 || h < bh + 2 { return; }
    let ptop = top + (h - bh) / 2;
    let pleft = left + (w - bw) / 2;
    let mut put = |row: u16, col: u16, c: char, bold: bool| {
        cells.push(CellView { col, row, c, fg: ink, bg, bold, italic: false });
    };
    for i in 0..inner {
        put(ptop, pleft + 1 + i, '\u{2550}', false);
        put(ptop + 2, pleft + 1 + i, '\u{2550}', false);
        let c = if (PAD..PAD + PLATE.len() as u16).contains(&i) {
            PLATE.as_bytes()[(i - PAD) as usize] as char
        } else {
            ' '
        };
        put(ptop + 1, pleft + 1 + i, c, c != ' ');
    }
    for (row, l, r) in [
        (ptop, '\u{2554}', '\u{2557}'),
        (ptop + 1, '\u{2551}', '\u{2551}'),
        (ptop + 2, '\u{255a}', '\u{255d}'),
    ] {
        put(row, pleft, l, false);
        put(row, pleft + bw - 1, r, false);
    }
}

/// One extra hint row when a saved session exists: `restore` carries the
/// snapshot's shell count (cleared once `/restore` spends it).
fn restore_hint(n: usize) -> String {
    format!(
        "{n} pane{} from last session    \u{00b7}    /restore",
        if n == 1 { "" } else { "s" }
    )
}

/// Render one animation frame: the rain field centred, tagline + hint below
/// it (plus a `/restore` hint when a session snapshot exists), version stamp
/// bottom-right. Falls back to a spaced single-line "CREW" when nothing
/// rain-sized fits. All cells stay within `cols × rows`.
// rustfmt::skip preserves compact inline struct literals.
#[rustfmt::skip]
pub fn welcome_cells_animated(cols: u16, rows: u16, tick: u64, restore: Option<usize>) -> Vec<CellView> {
    if cols == 0 || rows == 0 { return Vec::new(); }
    let mut cells = Vec::new();
    let t = crew_theme::theme();
    let bg = t.page_bg;

    if let Some(w) = rain_width(cols, rows) {
        let h = w / ASPECT;
        let top = (rows - (h + 3)) / 2;
        let left = (cols - w) / 2;
        // The rain falls INSIDE the frame (the box's outer ring), and the
        // CREW nameplate sits over its centre — glyphs stream around it.
        rain(&mut cells, top + 1, left + 1, w - 2, h - 2, tick, t.ink, t.text_muted, bg);
        frame(&mut cells, top, left, w, h, t.text_muted, bg);
        nameplate(&mut cells, top, left, w, h, t.ink, bg);

        let tl_row = top + h + 1;
        let tl_w = TAGLINE.chars().count() as u16;
        if tl_row < rows && tl_w < cols {
            push_str(&mut cells, tl_row, (cols - tl_w) / 2, TAGLINE, t.hint_fg, bg);
        }
        let hint_row = tl_row + 1;
        let hint_w = HINT.chars().count() as u16;
        if hint_row < rows && hint_w < cols {
            push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
        }
        if let Some(n) = restore {
            let line = restore_hint(n);
            let (row, w) = (hint_row + 2, line.chars().count() as u16);
            // `row + 1 < rows`: the bottom row belongs to the version stamp
            // (drawn after, last-write-wins) — skip rather than collide.
            if row + 1 < rows && w < cols {
                push_str(&mut cells, row, (cols - w) / 2, &line, t.hint_fg, bg);
            }
        }
    } else {
        // Fallback: spaced single-line "CREW" — same layout math as the old
        // figlet-era fallback, minus the deleted per-column shimmer (static ink).
        let letters: Vec<char> = "CREW".chars().collect();
        let span = (letters.len() as u16 - 1) * 2 + 1;
        if span < cols {
            let row   = rows / 2;
            let start = (cols - span) / 2;
            for (i, &ch) in letters.iter().enumerate() {
                cells.push(CellView { col: start + i as u16 * 2, row, c: ch, fg: t.ink, bg, bold: true, italic: false });
            }
            let hint_w   = HINT.chars().count() as u16;
            let hint_row = row + 2;
            if hint_w < cols && hint_row < rows {
                push_str(&mut cells, hint_row, (cols - hint_w) / 2, HINT, t.hint_fg, bg);
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
