//! Runs of a script that joins its letters — Devanagari and the other Indic
//! blocks, Thai, Lao, Myanmar, Khmer — drawn whole.
//!
//! A pane's own buffer snaps every glyph to a cell (`celltext`), which is
//! right for Latin and wrong for a script whose vowel signs and conjuncts are
//! parts of one shape: `भारत` came out `भ ा र त`. So a run of such cells is
//! left as spaces in that buffer, shaped here on its own at its natural
//! advance, and drawn as a text area of its own from its first cell, on the
//! row's baseline. The cells stay where the widths put them, and with them
//! the cursor, the selection, every background and every rule; only the ink
//! is drawn as one piece. WezTerm and Kitty do the same with HarfBuzz.
use std::sync::Arc;

use glyphon::Buffer;
use unicode_width::UnicodeWidthChar;

use crate::cellgrid::CellView;
use crate::scene::PaneBuffer;

pub(crate) use crate::scriptplace::place;

/// Whether `c` is a letter of a script whose letters join.
pub(crate) fn joins(c: char) -> bool {
    matches!(
        u32::from(c),
        0x0900..=0x0DFF // Devanagari, Bengali, Gurmukhi … Malayalam, Sinhala
            | 0x0E00..=0x0EFF // Thai, Lao
            | 0x1000..=0x109F // Myanmar
            | 0x1780..=0x17FF // Khmer
            | 0x19E0..=0x19FF // Khmer symbols
            | 0x1CD0..=0x1CFF // Vedic extensions
            | 0xA8E0..=0xA8FF // Devanagari extended
            | 0xA9E0..=0xA9FF // Myanmar extended-B
            | 0xAA60..=0xAA7F // Myanmar extended-A
    )
}

/// Punctuation that ends a word rather than standing apart: what a run takes
/// on when it comes straight after it. Not `|` or `/`, which a program draws
/// columns with.
const TRAILS: &str = ",.:;!?)]}'\"-\u{2026}\u{2019}\u{201d}";

/// One run as its cells give it: where it starts, how many cells its
/// characters' widths sum to, its text with every mark, and its one style.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RunSpec {
    pub row: u16,
    pub col: u16,
    pub cols: u16,
    pub text: String,
    pub fg: (u8, u8, u8),
    pub weight: u16,
    pub italic: bool,
    /// The coverage polarity and correction its cells would have shaped
    /// with (see `celltext::RunKey`), so it reads like the text beside it.
    pub light: bool,
    pub gamma: u8,
    /// Blank columns straight after it on its row, before the next glyph.
    pub spare: u16,
}

impl RunSpec {
    /// The width the run may take: its cells, and most of the blanks after
    /// it. A word wider than its cells — `आज` is two cells and nearly three
    /// cells of ink, Bengali and Tamil more — spills into the space after it
    /// rather than shrinking, and keeps half a cell clear of whatever comes
    /// next. It borrows at most its own width again.
    pub fn room(&self, cell_w: f32) -> f32 {
        let spill = f32::from(self.spare.min(self.cols)) * cell_w;
        f32::from(self.cols) * cell_w + (spill - cell_w / 2.0).max(0.0)
    }
}

/// The runs a fill pass found, in reading order.
#[derive(Default)]
pub(crate) struct Runs(pub Vec<RunSpec>);

impl Runs {
    /// Take `cell` into a run when its script joins: the run it continues —
    /// the next column, in the same style — or a new one. A style change
    /// splits the run, so a bold headline is bold and a link is the link's
    /// colour. Punctuation straight after a run goes on it, so `भेजा,` keeps
    /// its comma when the word is drawn off its first cell. `false` leaves
    /// the cell to the snapped buffer.
    pub fn take(&mut self, cell: &CellView, weight: u16, light: bool, gamma: u8) -> bool {
        let style = (cell.fg, weight, cell.italic, light, gamma);
        let goes_on = self.0.last().is_some_and(|r| {
            r.row == cell.row
                && r.col + r.cols == cell.col
                && (r.fg, r.weight, r.italic, r.light, r.gamma) == style
        });
        if !(joins(cell.c) || goes_on && TRAILS.contains(cell.c)) {
            return false;
        }
        let cols = UnicodeWidthChar::width(cell.c).unwrap_or(1).clamp(1, 2) as u16;
        let run = match self.0.last_mut() {
            Some(r) if goes_on => {
                r.cols += cols;
                r
            }
            _ => {
                self.0.push(RunSpec {
                    row: cell.row,
                    col: cell.col,
                    cols,
                    text: String::new(),
                    fg: cell.fg,
                    weight,
                    italic: cell.italic,
                    light,
                    gamma,
                    spare: 0,
                });
                self.0.last_mut().expect("just pushed")
            }
        };
        run.text.push(cell.c);
        run.text.extend(cell.marks.chars());
        true
    }

    /// A blank column at `(row, col)`: room for the run just before it.
    pub fn blank(&mut self, row: u16, col: u16) {
        if let Some(r) = self.0.last_mut() {
            if r.row == row && r.col + r.cols + r.spare == col {
                r.spare += 1;
            }
        }
    }
}

/// A run placed in its pane: drawn from `(dx, dy)` off the pane's origin, so
/// a pane that moved reuses it as it reuses its own buffer, `ink` wide. Its
/// cells, and the blanks after them, ride along for the rules it wears
/// (`scriptplace::Warp`).
#[derive(Clone)]
pub(crate) struct RunArea {
    pub buf: Arc<Buffer>,
    pub dx: f32,
    pub dy: f32,
    pub ink: f32,
    pub row: u16,
    pub col: u16,
    pub cols: u16,
    pub spare: u16,
}

/// Every buffer a pass draws, with its origin and the pane rect that clips
/// it (left, top, right, bottom): each pane's snapped buffer, then its runs.
pub(crate) fn areas(buffers: &[PaneBuffer]) -> impl Iterator<Item = (&Buffer, f32, f32, [f32; 4])> {
    buffers.iter().flat_map(|(buf, ox, oy, pw, ph, runs)| {
        let (ox, oy) = (*ox, *oy);
        let clip = [ox, oy, ox + pw, oy + ph];
        std::iter::once((buf, ox, oy, clip)).chain(
            runs.iter()
                .map(move |r| (&*r.buf, ox + r.dx, oy + r.dy, clip)),
        )
    })
}

#[cfg(test)]
#[path = "scriptrun_tests.rs"]
pub(crate) mod tests;
