//! The SYSTEM section drawn as three arc gauges — CPU, MEM, DISK as rings
//! with their reading in the hole and their name under it.
//!
//! The bar version ([`crate::gauges`]) is still there and still ships on a
//! narrow nav: three rings need columns a 12-column sidebar does not have,
//! and a gauge that does not fit is worse than a plainer one that does. Both
//! read the same tier colours and the same shape cues, so widening the nav
//! changes the shape of the answer and never the answer.
use crew_render::{CellView, Paint};

use crate::gauges::{fill_color, track_color};
use crate::plot::{gauge, Canvas};
use crate::stats::Stats;

/// Rows the ring block occupies: three for the rings (their reading sits on
/// the middle one, in the hole) and one for the names beneath.
///
/// A ring centred on a row *boundary* would put its hole across two text rows
/// and its number half over its own stroke — the reading has to sit on a row,
/// which costs the block a third ring row.
pub const ROWS: u16 = 4;
/// Columns one gauge claims, including the air to its right.
const SLOT: u16 = 6;
/// Interior columns below which the section falls back to bars.
pub const MIN_COLS: u16 = 3 + 3 * SLOT;

const R_OUT: f32 = 2.6;
const R_IN: f32 = 1.45;
/// Column the first ring's centre sits on — indented under the section legend
/// like every other line in the sidebar.
const FIRST_CX: f32 = 5.5;

/// Whether this nav width gets rings.
pub fn fits(cols: u16) -> bool {
    cols >= MIN_COLS
}

/// `(label, fraction)` for the three readings, in the order they are drawn.
fn readings(stats: Stats) -> [(&'static str, f32); 3] {
    [("cpu", stats.cpu), ("mem", stats.mem), ("dsk", stats.disk)]
}

/// Centre column of gauge `i`, in canvas units.
fn centre_x(i: usize) -> f32 {
    FIRST_CX + i as f32 * f32::from(SLOT)
}

/// The block's text: each reading as a percentage in its ring's hole, and the
/// three names on the row under them. `row0` is the block's first row.
pub fn cells(stats: Stats, row0: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    for (i, (label, frac)) in readings(stats).into_iter().enumerate() {
        let cx = centre_x(i);
        // Two digits, no percent sign: the hole is 2.5 columns across and the
        // sign says nothing the ring has not already said. 100% reads "99"
        // nowhere — it reads "100", and is allowed to fill its hole.
        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
        let pct = pct.to_string();
        write_centred(&mut out, &pct, cx, row0 + 1, t.ink, t.page_bg);
        // The tier cue rides with the name, in the ring's own colour: the band
        // is said in shape as well as hue, as it is beside the bars.
        let mark = crate::shapecues::Tier::of(frac).mark();
        let name = match mark {
            Some(m) => format!("{label}{m}"),
            None => label.to_string(),
        };
        let fg = match mark {
            Some(_) => fill_color(frac),
            None => t.text_muted,
        };
        write_centred(&mut out, &name, cx, row0 + 3, fg, t.page_bg);
    }
    out
}

/// The three rings.
pub fn paint(stats: Stats, cols: u16, row0: u16, aspect: f32) -> Vec<Paint> {
    if !fits(cols) {
        return Vec::new();
    }
    let mut c = Canvas::new(cols, ROWS, aspect);
    // Centred on the middle ring row — the row the reading is written on, so
    // the number lands in the hole — leaving the fourth row clear for names.
    let cy = 1.5 * aspect;
    for (i, (_, frac)) in readings(stats).into_iter().enumerate() {
        gauge::arc(
            &mut c,
            (centre_x(i), cy),
            R_OUT,
            R_IN,
            frac,
            fill_color(frac),
            track_color(),
        );
    }
    c.paint()
        .into_iter()
        .map(|p| p.shifted(0.0, f32::from(row0)))
        .collect()
}

/// Write `text` centred on column `cx` (a canvas unit, i.e. a column).
fn write_centred(
    out: &mut Vec<CellView>,
    text: &str,
    cx: f32,
    row: u16,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    let n = text.chars().count() as f32;
    let start = (cx - n / 2.0).round().max(0.0) as u16;
    for (i, ch) in text.chars().enumerate() {
        out.push(CellView {
            col: start + i as u16,
            row,
            c: ch,
            fg,
            bg,
            ..Default::default()
        });
    }
}
