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
/// Columns one gauge claims at the narrowest nav that gets rings at all,
/// including the air to its right.
const SLOT: u16 = 6;
/// …and the widest it will spread to. Past this the three stop reading as one
/// group and start reading as three unrelated dials: they answer one question
/// together, so they stay together and the group centres in the extra width
/// instead of being stretched across it.
const MAX_SLOT: f32 = 8.0;
/// Interior columns below which the section falls back to bars.
pub const MIN_COLS: u16 = 3 + 3 * SLOT;

const R_OUT: f32 = 2.6;
const R_IN: f32 = 1.45;

/// Whether this nav width gets rings.
pub fn fits(cols: u16) -> bool {
    cols >= MIN_COLS
}

/// `(label, fraction)` for the three readings, in the order they are drawn.
fn readings(stats: Stats) -> [(&'static str, f32); 3] {
    [("cpu", stats.cpu), ("mem", stats.mem), ("dsk", stats.disk)]
}

/// Centre column of gauge `i` in a `cols`-wide nav, in canvas units.
///
/// The three share the content width up to [`MAX_SLOT`] each, and the group
/// is then centred in whatever is left over. Dragged wide, the rings used to
/// stay pinned at the left edge with a third of the section empty beside
/// them; stretched to fill it they stop looking like one reading in three
/// parts.
fn centre_x(i: usize, cols: u16) -> f32 {
    let avail = f32::from(cols.saturating_sub(crate::navtext::INDENT + 1)).max(1.0);
    let slot = (avail / 3.0).clamp(f32::from(SLOT), MAX_SLOT);
    let left = f32::from(crate::navtext::INDENT) + (avail - slot * 3.0).max(0.0) / 2.0;
    left + (i as f32 + 0.5) * slot
}

/// The block's text: each reading as a percentage in its ring's hole, and the
/// three names on the row under them. `row0` is the block's first row.
pub fn cells(stats: Stats, cols: u16, row0: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    for (i, (label, frac)) in readings(stats).into_iter().enumerate() {
        let cx = centre_x(i, cols);
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
            (centre_x(i, cols), cy),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> Stats {
        Stats {
            cpu: 0.24,
            mem: 0.60,
            disk: 0.77,
            ..Default::default()
        }
    }

    /// Dragged wide, the rings used to stay pinned at the left with a third
    /// of the section empty beside them. They spread — up to MAX_SLOT — and
    /// then the group centres in whatever is left, so they keep reading as
    /// one answer in three parts instead of three unrelated dials.
    #[test]
    fn the_rings_spread_then_centre_instead_of_hugging_the_left_edge() {
        let span = |cols| centre_x(2, cols) - centre_x(0, cols);
        assert!(span(37) > span(21), "{} vs {}", span(37), span(21));
        assert!(
            (span(80) - span(37)).abs() < 1e-3,
            "and stop spreading at the cap: {} vs {}",
            span(80),
            span(37)
        );
        // Centred: the air left of the first ring matches the air right of
        // the last, within a column.
        let cols = 60u16;
        let left = centre_x(0, cols) - R_OUT - f32::from(crate::navtext::INDENT);
        let right = f32::from(cols - 1) - (centre_x(2, cols) + R_OUT);
        assert!((left - right).abs() < 1.5, "left {left}, right {right}");
    }

    /// …and at the narrowest nav that still gets rings they sit where they
    /// always did: indented under the legend, one slot each, nothing wasted.
    #[test]
    fn the_narrowest_ring_nav_keeps_the_slot_it_was_built_for() {
        assert!(fits(MIN_COLS) && !fits(MIN_COLS - 1));
        let gap = centre_x(1, MIN_COLS) - centre_x(0, MIN_COLS);
        assert!((gap - f32::from(SLOT)).abs() < 1e-3, "gap {gap}");
        assert!(centre_x(0, MIN_COLS) >= f32::from(crate::navtext::INDENT));
    }

    /// The reading lands in its own ring's hole at every width — the number
    /// and the arc are drawn by two different passes off one `centre_x`.
    #[test]
    fn every_reading_sits_in_its_own_ring() {
        let _g = crate::app::theme_test_guard();
        for cols in [MIN_COLS, 26, 37, 60] {
            let cells = cells(stats(), cols, 0);
            for (i, want) in ["24", "60", "77"].into_iter().enumerate() {
                let cx = centre_x(i, cols);
                let text: String = {
                    let mut v: Vec<_> = cells
                        .iter()
                        .filter(|c| c.row == 1 && (f32::from(c.col) - cx).abs() <= R_OUT)
                        .collect();
                    v.sort_by_key(|c| c.col);
                    v.iter().map(|c| c.c).collect()
                };
                assert_eq!(text, want, "cols={cols} ring {i}");
            }
        }
    }
}
