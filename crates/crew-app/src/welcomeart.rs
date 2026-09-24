//! The welcome screen's drawing primitives: the CREW nameplate over the
//! rain, and the two ways text lands on the canvas — one
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

/// The `C R E W` wordmark centred in the rain: bold ink letters standing in
/// a clearing — every cell of it pushed, blanks included, so it occludes
/// the rain behind it (crew-render's last-write-wins merge) and the glyphs
/// fall AROUND the name. It was a double-ruled plate; the name needs no box.
/// Skipped when the box hasn't the room to hold it with a rain margin.
#[rustfmt::skip]
pub(crate) fn nameplate(cells: &mut Vec<CellView>, top: u16, left: u16, w: u16, h: u16, ink: (u8,u8,u8), bg: (u8,u8,u8)) {
    const PLATE: &str = "C R E W";
    const PAD: u16 = 4;
    let (bw, bh) = (PLATE.len() as u16 + PAD * 2, 3u16);
    if w < bw + 4 || h < bh + 2 { return; }
    let (ptop, pleft) = (top + (h - bh) / 2, left + (w - bw) / 2);
    for row in ptop..ptop + bh {
        for i in 0..bw {
            let c = match row == ptop + 1 && (PAD..PAD + PLATE.len() as u16).contains(&i) {
                true => PLATE.as_bytes()[(i - PAD) as usize] as char,
                false => ' ',
            };
            cells.push(CellView { col: pleft + i, row, c, fg: ink, bg, bold: c != ' ', ..Default::default() });
        }
    }
}
