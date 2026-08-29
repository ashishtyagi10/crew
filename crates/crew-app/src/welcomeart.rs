//! The welcome screen's drawing primitives: the rain box's frame, the CREW
//! nameplate inside it, and the two ways text lands on the canvas — one
//! colour for a whole string, or a per-character run so the opening hint can
//! put its chords in the accent. Split from [`crate::welcome`] (which owns
//! the layout) for the 200-line cap.
use crew_render::CellView;

use crate::welcometext::{chord_fg, hint_for, hint_spans};

/// Push every character of `s` as cells starting at `(col, row)`.
// rustfmt::skip keeps the CellView struct literal on one line.
#[rustfmt::skip]
pub(crate) fn push_str(cells: &mut Vec<CellView>, row: u16, col: u16, s: &str, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    for (i, ch) in s.chars().enumerate() {
        cells.push(CellView { col: col + i as u16, row, c: ch, fg, bg, bold: false, italic: false, ..Default::default() });
    }
}

/// Push a run of already-coloured chars as cells starting at `(col, row)`.
#[rustfmt::skip]
pub(crate) fn push_spans(cells: &mut Vec<CellView>, row: u16, col: u16, spans: &[(char, (u8,u8,u8))], bg: (u8,u8,u8)) {
    for (i, &(c, fg)) in spans.iter().enumerate() {
        cells.push(CellView { col: col + i as u16, row, c, fg, bg, bold: false, italic: false, ..Default::default() });
    }
}

/// Draw the opening hint centred on `row`, with its chords in the accent —
/// the one line on this screen that says what to press.
pub(crate) fn push_hint(
    cells: &mut Vec<CellView>,
    row: u16,
    cols: u16,
    word: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let Some(hint) = hint_for(cols) else { return };
    let spans = hint_spans(hint, chord_fg(), word);
    push_spans(cells, row, (cols - spans.len() as u16) / 2, &spans, bg);
}

/// The rectangular frame on the rain box's outer ring: a muted single-line
/// border, so the rain reads as a bounded field rather than loose glyphs.
#[rustfmt::skip]
pub(crate) fn frame(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, fg: (u8,u8,u8), bg: (u8,u8,u8)) {
    if w < 2 || h < 2 { return; }
    let (bot, right) = (top + h - 1, left + w - 1);
    let mut put = |row: u16, col: u16, c: char| {
        cells.push(CellView { col, row, c, fg, bg, bold: false, italic: false, ..Default::default() });
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
pub(crate) fn nameplate(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, ink: (u8,u8,u8), bg: (u8,u8,u8)) {
    const PLATE: &str = "C R E W";
    const PAD: u16 = 3;
    let inner = PLATE.len() as u16 + PAD * 2;
    let (bw, bh) = (inner + 2, 3u16);
    if w < bw + 4 || h < bh + 2 { return; }
    let ptop = top + (h - bh) / 2;
    let pleft = left + (w - bw) / 2;
    let mut put = |row: u16, col: u16, c: char, bold: bool| {
        cells.push(CellView { col, row, c, fg: ink, bg, bold, italic: false, ..Default::default() });
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
