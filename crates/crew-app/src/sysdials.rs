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

/// Rows the dial block occupies: three for the faces and one for the names.
///
/// The reading sits on the third of those rows, inside the face's own open
/// bottom — where a car's speedometer puts its odometer. The ring this
/// section used to draw put the number in the hole instead, with the stroke a
/// third of a column away on both sides.
pub const ROWS: u16 = 4;
/// Columns one dial claims at the narrowest nav that gets them at all,
/// including the air to its right.
const SLOT: u16 = 6;
/// …and the widest it will spread to. Past this the three stop reading as one
/// group and start reading as three unrelated instruments: they answer one
/// question together, so they stay together and the group centres in the extra
/// width instead of being stretched across it.
const MAX_SLOT: f32 = 8.0;
/// Interior columns below which the section falls back to bars.
pub const MIN_COLS: u16 = 3 + 3 * SLOT;

/// Canvas pixels per column. The default ([`crate::plot::canvas::SUB`]) is
/// four, which puts a canvas pixel at about *two* device pixels — a grid
/// coarser than the screen, which an area chart's roof survives and a dial's
/// needle does not: it steps in visible blocks whatever the coverage maths
/// says, because the block is the smallest thing the canvas can address.
///
/// Eight is one canvas pixel per device pixel at the default font, and shot
/// side by side against 4, 6 and 16 it is where the stepping stops; 16 is not
/// visibly better and costs twice the quads. The whole block is ~1400 of them
/// — `the_three_faces_cost_the_frame_a_bounded_number_of_quads` holds the
/// line.
const SUB: usize = 8;

/// The face's radius, in columns, for a `slot`-wide slot. Capped by the three
/// rows the block has: a dial taller than its rows would have its scale
/// clipped by the section above.
fn radius(slot: f32) -> f32 {
    (slot * 0.5 - 0.15).min(3.0)
}

/// Where the face's centre sits, in canvas units down from the block's top:
/// the middle of the three dial rows.
///
/// The face is as large as those rows allow on purpose. A smaller one leaves
/// the digits hanging off its bottom edge with the plate cutting across their
/// tops; at full height the open third of the scale is a window the number
/// sits *inside*, which is where an instrument keeps its odometer.
const CY: f32 = 3.0;

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

/// How wide one dial's slot is on a `cols`-wide nav.
fn slot_w(cols: u16) -> f32 {
    (avail(cols) / 3.0).clamp(f32::from(SLOT), MAX_SLOT)
}

/// Content width the three share, in columns.
fn avail(cols: u16) -> f32 {
    f32::from(cols.saturating_sub(crate::navtext::INDENT + 1)).max(1.0)
}

/// Centre column of dial `i` in a `cols`-wide nav, in canvas units.
///
/// The three share the content width up to [`MAX_SLOT`] each, and the group
/// is then centred in whatever is left over. Dragged wide, the dials used to
/// stay pinned at the left edge with a third of the section empty beside
/// them; stretched to fill it they stop looking like one reading in three
/// parts.
fn centre_x(i: usize, cols: u16) -> f32 {
    let slot = slot_w(cols);
    let left = f32::from(crate::navtext::INDENT) + (avail(cols) - slot * 3.0).max(0.0) / 2.0;
    left + (i as f32 + 0.5) * slot
}

/// The block's text: each reading as a percentage in the gap at the bottom of
/// its own face, and the three names on the row under them. `row0` is the
/// block's first row.
pub fn cells(stats: Stats, cols: u16, row0: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    for (i, (label, frac)) in readings(stats).into_iter().enumerate() {
        let cx = centre_x(i, cols);
        // Two digits, no percent sign: the gap is about two columns across
        // and the sign says nothing the needle has not already said. 100%
        // reads "99" nowhere — it reads "100", and the gap widens toward the
        // rim, which is the direction a third digit grows.
        let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
        write_centred(&mut out, &pct.to_string(), cx, row0 + 2, t.ink, t.page_bg);
        // The tier cue rides with the name, in the dial's own colour: the band
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

/// The three faces.
pub fn paint(stats: Stats, cols: u16, row0: u16, aspect: f32) -> Vec<Paint> {
    if !fits(cols) {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let (track, track_dim) = scale_colors();
    let mut c = Canvas::with_sub(cols, ROWS, aspect, SUB);
    let r = radius(slot_w(cols));
    for (i, (_, frac)) in readings(stats).into_iter().enumerate() {
        dial::draw(
            &mut c,
            Dial {
                centre: (centre_x(i, cols), CY),
                r,
                frac,
                color: fill_color(frac),
                track,
                track_dim,
                // The face is barely there: enough to lift the ticks off the
                // paper, faint enough that three of them do not read as three
                // grey coins on the page.
                plate: Some((t.ink, 0.06)),
            },
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
    fn the_dials_spread_then_centre_instead_of_hugging_the_left_edge() {
        let span = |cols| centre_x(2, cols) - centre_x(0, cols);
        assert!(span(37) > span(21), "{} vs {}", span(37), span(21));
        assert!(
            (span(80) - span(37)).abs() < 1e-3,
            "and stop spreading at the cap: {} vs {}",
            span(80),
            span(37)
        );
        // Centred: the air left of the first face matches the air right of
        // the last, within a column.
        let cols = 60u16;
        let r = radius(slot_w(cols));
        let left = centre_x(0, cols) - r - f32::from(crate::navtext::INDENT);
        let right = f32::from(cols - 1) - (centre_x(2, cols) + r);
        assert!((left - right).abs() < 1.5, "left {left}, right {right}");
    }

    /// What the block costs the frame. Every rectangle here is a quad the
    /// GPU draws on top of the ~1500 the cell backgrounds already push, and
    /// this section redraws on the sampler's every second — so the number is
    /// worth knowing rather than assuming. Three faces at sixteen canvas
    /// pixels to the column is the resolution that stopped the arc
    /// staircasing; if it ever costs thousands of quads, the resolution is
    /// the knob to turn, not the shapes.
    #[test]
    fn the_three_faces_cost_the_frame_a_bounded_number_of_quads() {
        let _g = crate::app::theme_test_guard();
        for cols in [MIN_COLS, 26, 37, 60] {
            let n = paint(stats(), cols, 1, 2.0).len();
            assert!(n < 1600, "cols={cols}: {n} quads");
        }
    }

    /// The scale has to be visible on the page it is drawn on, or the dial is
    /// a needle pointing at nothing. The palette's own border shade is not:
    /// on the light themes it lands near 1.2 against the page, which is what
    /// the first light-theme shot of this section showed — three faces with a
    /// hand and no marks. Both scale colours are derived against the page,
    /// and both are laid down opaque, so what is measured here is what is
    /// drawn.
    #[test]
    fn the_scale_reads_on_every_page() {
        let _g = crate::app::theme_test_guard();
        let floor = crew_theme::contrast::mark_floor();
        for id in crew_theme::ALL_THEMES {
            crew_theme::set_theme(id);
            let page = crew_theme::theme().page_bg;
            let (major, minor) = scale_colors();
            for (what, c) in [("major", major), ("minor", minor)] {
                let cr = crew_theme::contrast_ratio(c, page);
                assert!(cr >= floor - 0.01, "{id:?} {what} tick at {cr:.2}");
            }
            // …and the ranking survives: a minor tick is never louder than a
            // major one, however little headroom the page leaves.
            assert!(
                crew_theme::contrast_ratio(minor, page)
                    <= crew_theme::contrast_ratio(major, page) + 0.01,
                "{id:?} minor ticks outshout the majors"
            );
        }
    }

    /// …and at the narrowest nav that still gets dials they sit where they
    /// always did: indented under the legend, one slot each, nothing wasted.
    #[test]
    fn the_narrowest_dial_nav_keeps_the_slot_it_was_built_for() {
        assert!(fits(MIN_COLS) && !fits(MIN_COLS - 1));
        let gap = centre_x(1, MIN_COLS) - centre_x(0, MIN_COLS);
        assert!((gap - f32::from(SLOT)).abs() < 1e-3, "gap {gap}");
        assert!(centre_x(0, MIN_COLS) >= f32::from(crate::navtext::INDENT));
    }

    /// The reading lands in the gap of its own face at every width — the
    /// number and the needle are drawn by two different passes off one
    /// `centre_x`.
    #[test]
    fn every_reading_sits_in_its_own_face() {
        let _g = crate::app::theme_test_guard();
        for cols in [MIN_COLS, 26, 37, 60] {
            let cells = cells(stats(), cols, 0);
            for (i, want) in ["24", "60", "77"].into_iter().enumerate() {
                let cx = centre_x(i, cols);
                let text: String = {
                    let mut v: Vec<_> = cells
                        .iter()
                        .filter(|c| {
                            c.row == 2 && (f32::from(c.col) - cx).abs() <= radius(slot_w(cols))
                        })
                        .collect();
                    v.sort_by_key(|c| c.col);
                    v.iter().map(|c| c.c).collect()
                };
                assert_eq!(text, want, "cols={cols} dial {i}");
            }
        }
    }
}
