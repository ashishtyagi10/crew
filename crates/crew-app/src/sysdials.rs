//! The SYSTEM section drawn as three instrument dials — CPU, MEM, DISK as
//! needles on ticked faces, with the reading in the gap at the bottom of each
//! face and its name under it.
//!
//! The bar version ([`crate::gauges`]) is still there and still ships on a
//! narrow nav: three dials need columns a 12-column sidebar does not have,
//! and a gauge that does not fit is worse than a plainer one that does. Both
//! read the same tier colours and the same shape cues, so widening the nav
//! changes the shape of the answer and never the answer.
use crew_render::{CellView, Paint};

use crate::gauges::fill_color;
use crate::plot::dial::{self, Dial};
use crate::plot::Canvas;
use crate::stats::Stats;

/// Rows the block occupies in the nav: three for the faces and one for the
/// names.
///
/// The reading sits on the last of the face's own rows, inside its open
/// bottom — where a car's speedometer puts its odometer. The ring this
/// section used to draw put the number in the hole instead, with the stroke a
/// third of a column away on both sides.
pub const ROWS: u16 = NAV.rows;
/// Columns one dial claims at the narrowest nav that gets them at all,
/// including the air to its right.
const SLOT: u16 = 6;
/// Interior columns below which the section falls back to bars.
pub const MIN_COLS: u16 = 3 + 3 * SLOT;

/// How much room the three dials are being given.
///
/// The nav's block is four rows and its faces are capped narrow: they sit in
/// a column beside a pane list, and three dials stretched across a dragged-out
/// sidebar stop reading as one answer in three parts. The dashboard is the
/// same three readings at a size worth looking at — it has the rows, and its
/// whole reason to exist is showing these widgets larger than the nav can.
#[derive(Debug, Clone, Copy)]
pub struct Dials {
    /// Rows the whole block claims, the name row included.
    pub rows: u16,
    /// The widest one dial's slot may grow to, in columns. Past this the
    /// three stop reading as one group and start reading as three unrelated
    /// instruments, so they stay together and the group centres in the extra
    /// width instead of being stretched across it.
    pub max_slot: f32,
}

/// The sidebar's: four rows, faces capped at eight columns.
pub const NAV: Dials = Dials {
    rows: 4,
    max_slot: 8.0,
};
/// The dashboard's: six rows, faces twice the nav's across.
///
/// The slot is wider than the face needs. At this size the height is what
/// caps the radius, so the extra columns become air *between* the faces —
/// without it their plates come within two thirds of a column of touching
/// and the three read as one wide smear rather than three instruments.
pub const DASH: Dials = Dials {
    rows: 6,
    max_slot: 12.0,
};
/// Columns the dashboard's block claims — three of its slots, plus the
/// indent every section is written under.
pub const DASH_COLS: u16 = 3 + 3 * 12;

/// The scale's two colours: the bezel and major ticks, and the minor ticks a
/// rank under them.
///
/// Derived against the page rather than taken from the palette. The track the
/// ring gauge used — the theme's recessed border shade — reads at about 3 on
/// every dark page and under 1.3 on every light one, which on the light
/// themes drew a dial with a needle and no scale to read it against. A scale
/// nobody can see is the whole widget wasted, so these clear the mark floor
/// by construction and `the_scale_reads_on_every_page` measures that they do.
fn scale_colors() -> ((u8, u8, u8), (u8, u8, u8)) {
    let t = crew_theme::theme();
    let major = crew_theme::readable::against(
        t.border_normal,
        t.page_bg,
        crew_theme::contrast::mark_floor(),
    );
    (major, crew_theme::readable::secondary(major, t.page_bg))
}

/// Whether this nav width gets dials.
pub fn fits(cols: u16) -> bool {
    cols >= MIN_COLS
}

/// `(label, fraction)` for the three readings, in the order they are drawn.
fn readings(stats: Stats) -> [(&'static str, f32); 3] {
    [("cpu", stats.cpu), ("mem", stats.mem), ("dsk", stats.disk)]
}

/// Content width the three share, in columns.
fn avail(cols: u16) -> f32 {
    f32::from(cols.saturating_sub(crate::navtext::INDENT + 1)).max(1.0)
}

impl Dials {
    /// Rows the faces themselves get: everything but the name row.
    fn face_rows(&self) -> u16 {
        self.rows.saturating_sub(1)
    }

    /// Where a face's centre sits, in canvas units down from the block's top:
    /// the middle of its own rows.
    fn cy(&self, aspect: f32) -> f32 {
        f32::from(self.face_rows()) * aspect * 0.5
    }

    /// How wide one dial's slot is at this width.
    fn slot_w(&self, cols: u16) -> f32 {
        (avail(cols) / 3.0).clamp(f32::from(SLOT), self.max_slot)
    }

    /// The face's radius, in columns. Whichever runs out first — the slot's
    /// width or the block's height — decides it.
    ///
    /// The face fills its rows on purpose. A smaller one leaves the digits
    /// hanging off its bottom edge with the plate cutting across their tops;
    /// at full height the open third of the scale is a window the number sits
    /// *inside*, which is where an instrument keeps its odometer.
    fn radius(&self, cols: u16, aspect: f32) -> f32 {
        (self.slot_w(cols) * 0.5 - 0.15).min(self.cy(aspect))
    }

    /// The row the digits go on: the one nearest 0.72 of the face's radius
    /// below its centre, which is where an instrument keeps its odometer.
    ///
    /// Derived from the radius rather than counted from the bottom of the
    /// block. A dashboard block squeezed narrow keeps its rows but loses
    /// radius, and digits pinned to the last face row then sit *outside* the
    /// face they are supposed to be a window in.
    fn digits_row(&self, cols: u16) -> u16 {
        // A cell's aspect only picks a row here, and every monospace face
        // this app will load is within a tenth of two.
        let a = 2.0;
        let (cy, r) = (self.cy(a), self.radius(cols, a));
        let row_at = |depth: f32| (cy + depth * r) / a - 0.5;
        // The nearest row to the window's place, but never past the last one
        // that is still *on* the face: rounding to the nearest row can cross
        // the rim on a face that is narrower than its rows are tall.
        let want = row_at(0.72).round().min(row_at(0.85).floor());
        (want.max(1.0) as u16).min(self.face_rows().saturating_sub(1))
    }

    /// Centre column of dial `i` at this width, in canvas units.
    ///
    /// The three share the content width up to [`Dials::max_slot`] each, and
    /// the group is then centred in whatever is left over. Dragged wide, the
    /// dials used to stay pinned at the left edge with a third of the section
    /// empty beside them; stretched to fill it they stop looking like one
    /// reading in three parts.
    fn centre_x(&self, i: usize, cols: u16) -> f32 {
        let slot = self.slot_w(cols);
        let left = f32::from(crate::navtext::INDENT) + (avail(cols) - slot * 3.0).max(0.0) / 2.0;
        left + (i as f32 + 0.5) * slot
    }

    /// The block's text: each reading as a percentage in the gap at the bottom
    /// of its own face, and the three names on the row under them. `row0` is
    /// the block's first row.
    pub fn cells(&self, stats: Stats, cols: u16, row0: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        let mut out = Vec::new();
        let digits = row0 + self.digits_row(cols);
        for (i, (label, frac)) in readings(stats).into_iter().enumerate() {
            let cx = self.centre_x(i, cols);
            // Two digits, no percent sign: the gap is about two columns across
            // and the sign says nothing the needle has not already said. 100%
            // reads "99" nowhere — it reads "100", and the gap widens toward
            // the rim, which is the direction a third digit grows.
            let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
            write_centred(&mut out, &pct.to_string(), cx, digits, t.ink, t.page_bg);
            // The tier cue rides with the name, in the dial's own colour: the
            // band is said in shape as well as hue, as it is beside the bars.
            let mark = crate::shapecues::Tier::of(frac).mark();
            let name = match mark {
                Some(m) => format!("{label}{m}"),
                None => label.to_string(),
            };
            let fg = match mark {
                Some(_) => fill_color(frac),
                None => t.text_muted,
            };
            write_centred(&mut out, &name, cx, row0 + self.rows - 1, fg, t.page_bg);
        }
        out
    }

    /// The three faces.
    pub fn paint(&self, stats: Stats, cols: u16, row0: u16, aspect: f32) -> Vec<Paint> {
        if !fits(cols) {
            return Vec::new();
        }
        let t = crew_theme::theme();
        let (track, track_dim) = scale_colors();
        let mut c = Canvas::new(cols, self.face_rows(), aspect);
        let r = self.radius(cols, aspect);
        for (i, (_, frac)) in readings(stats).into_iter().enumerate() {
            dial::draw(
                &mut c,
                Dial {
                    centre: (self.centre_x(i, cols), self.cy(aspect)),
                    r,
                    frac,
                    color: fill_color(frac),
                    track,
                    track_dim,
                    // The face is barely there: enough to lift the ticks off
                    // the paper, faint enough that three of them do not read
                    // as three grey coins on the page.
                    plate: Some((t.ink, 0.06)),
                },
            );
        }
        c.paint()
            .into_iter()
            .map(|p| p.shifted(0.0, f32::from(row0)))
            .collect()
    }
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
#[path = "sysdials_tests.rs"]
mod tests;
