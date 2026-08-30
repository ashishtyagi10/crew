//! Box-drawing and block glyphs, drawn as pixels instead of read from a font.
//!
//! Everything crew frames itself with is one of these characters: the cards'
//! `╭─╮│╰╯`, the sidebar's section rules, the meters' `▍`, the shaded fills
//! of a heatmap. A font has outlines for all of them, and a font is the wrong
//! place to get them from — three separate reasons, all measurable:
//!
//! * The outline is drawn at whatever weight the *typeface* chose, at
//!   whatever position in its em box, so a rule lands wherever the fractional
//!   arithmetic puts it and is antialiased across two pixels at each edge.
//! * Crew then runs it through the CoreText-style stem darkening in
//!   [`crate::smoothmask`] and the coverage curve in [`crate::textgamma`].
//!   Both are calibrated for *letterforms* — a curve wants its flanks filled
//!   out. A rule is a rectangle and wants nothing of the sort: the dilation
//!   spills a hairline into its neighbours and the curve lifts the spill.
//!   Measured on a card frame, one `─` spanned FOUR rows of pixels with a
//!   single row at full ink (0.20 / 0.78 / 1.00 / 0.25).
//! * Two adjacent cells' rules are rasterized independently, so a long run of
//!   `─` can wobble by a pixel and a `├` need not meet the `│` above it.
//!
//! Drawing them instead is what every native terminal does (ghostty, kitty,
//! WezTerm all synthesize this range). Here a glyph is a rectangle list in
//! the CELL's own box, snapped to whole pixels, so a rule is exactly as thick
//! as it says, sits on a pixel boundary, and every cell in a row draws the
//! identical bitmap. [`synth`] returns it as a ready `SwashImage`, which
//! [`crate::smoothing::presmooth`] seeds straight into the shared cache —
//! skipping the darkening and the curve, which is the whole point.
mod arms;
mod blocks;
mod braille;
mod doubles;
mod marks;
mod round;
mod sextants;
mod strokes;

use glyphon::cosmic_text::{Placement, SwashImage};

/// Samples per axis inside one pixel when integrating a curved or diagonal
/// edge. Shared by the rounded corners and the geometric marks.
pub(crate) const SUB: u32 = 8;

/// Largest cell box [`synth`] will allocate a mask for, per axis. A cell is a
/// character on a screen; anything past this is not a cell but a bad number,
/// and the shaper hands out bad numbers — a fallback face with an INFINITE
/// advance (the GB18030 Bitmap CJK quirk `celltext::cell_correction_em`
/// already sidesteps) reaches `glyph.w.round() as u32` as `u32::MAX`, and
/// `w * h` for the mask then overflows. One Japanese character in an agent's
/// reply was enough.
const MAX_CELL: u32 = 512;

/// A cell-sized coverage mask under construction.
pub(crate) struct Mask {
    pub(crate) w: u32,
    pub(crate) h: u32,
    pub(crate) data: Vec<u8>,
}

impl Mask {
    fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            data: vec![0; (w * h) as usize],
        }
    }

    /// Union a rectangle in — analytic coverage, so a whole-pixel rectangle
    /// (which is what the snapped geometry produces) comes out at a flat 255
    /// with nothing on its flanks, and a deliberately fractional one still
    /// antialiases correctly rather than snapping and lying about its weight.
    pub(crate) fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.rect_at(x0, y0, x1, y1, 1.0);
    }

    /// [`Mask::rect`] at a fraction of full coverage — what the shade
    /// characters are.
    pub(crate) fn rect_at(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, a: f32) {
        for py in 0..self.h {
            let ay = (y1.min(py as f32 + 1.0) - y0.max(py as f32)).max(0.0);
            if ay <= 0.0 {
                continue;
            }
            for px in 0..self.w {
                let ax = (x1.min(px as f32 + 1.0) - x0.max(px as f32)).max(0.0);
                if ax <= 0.0 {
                    continue;
                }
                let cov = (ax * ay * a * 255.0).round() as u8;
                let i = (py * self.w + px) as usize;
                self.data[i] = self.data[i].max(cov);
            }
        }
    }

    /// Union in whatever `inside` covers, integrated over `SUB`×`SUB` points
    /// per pixel. Curves and diagonals are the only shapes here that need it
    /// — a rectangle's coverage is computed outright by [`Mask::rect`] — and
    /// the mask is cached for the life of an atlas entry, so this can afford
    /// to be generous.
    pub(crate) fn sample(&mut self, inside: impl Fn(f32, f32) -> bool) {
        let step = 1.0 / SUB as f32;
        for py in 0..self.h {
            for px in 0..self.w {
                let mut hits = 0u32;
                for sy in 0..SUB {
                    let y = py as f32 + (sy as f32 + 0.5) * step;
                    for sx in 0..SUB {
                        let x = px as f32 + (sx as f32 + 0.5) * step;
                        if inside(x, y) {
                            hits += 1;
                        }
                    }
                }
                if hits > 0 {
                    let cov = (hits * 255 / (SUB * SUB)) as u8;
                    let i = (py * self.w + px) as usize;
                    self.data[i] = self.data[i].max(cov);
                }
            }
        }
    }

    /// Union in a stroked line segment of width `t`, in pixel coordinates.
    /// The one primitive the pictographic marks are all built from — a check
    /// is two of these, a cross is two, a chevron is two.
    pub(crate) fn stroke(&mut self, p0: (f32, f32), p1: (f32, f32), t: f32) {
        let (dx, dy) = (p1.0 - p0.0, p1.1 - p0.1);
        let len2 = dx * dx + dy * dy;
        let half = t / 2.0;
        self.sample(move |x, y| {
            // Distance to the segment, capped ends: the caps are what make
            // two strokes meet cleanly at a corner.
            let s = if len2 <= 0.0 {
                0.0
            } else {
                (((x - p0.0) * dx + (y - p0.1) * dy) / len2).clamp(0.0, 1.0)
            };
            let (qx, qy) = (p0.0 + s * dx, p0.1 + s * dy);
            ((x - qx).powi(2) + (y - qy).powi(2)).sqrt() <= half
        });
    }

    /// Whether anything was drawn — a character this module claims but which
    /// rounds away to nothing at this cell size is better left to the font.
    fn inked(&self) -> bool {
        self.data.iter().any(|v| *v > 0)
    }
}

/// The stroke width of a *light* line in a cell this tall.
///
/// The same answer [`crate::deco`] gives an underline, and deliberately so: a
/// `─` and an underlined word in the same pane are both rules, and there is no
/// reading of "crisp" under which they should be different weights.
///
/// It also has to track the font, not step once. This was `ch / 16`
/// TRUNCATING, which is one pixel from a 16-pixel cell all the way to a
/// 31-pixel one — so every font size from 13 up to 25 got the same hairline
/// while the text it framed nearly doubled, and a card at a display size read
/// as a thin wire around big letters. Rounding the same ratio steps it where
/// the letters' own stems step.
pub(crate) fn light_thickness(ch: u32) -> u32 {
    crate::deco::thickness(ch as f32) as u32
}

/// The pixel span `[lo, lo + t)` that centres a `t`-thick stroke in `extent`,
/// biased so it lands on whole pixels — a rule that straddles a pixel
/// boundary is the soft rule this module exists to replace.
pub(crate) fn centre(extent: u32, t: u32) -> (u32, u32) {
    let lo = extent.saturating_sub(t) / 2;
    (lo, lo + t)
}

/// Draw `c` into a `cw`×`ch` cell, or `None` when this module has nothing to
/// say about it and the font should answer. `top` is the placement's baseline
/// offset — the caller measures it from the layout run so the mask covers
/// exactly the cell the character was laid into.
pub(crate) fn synth(c: char, cw: u32, ch: u32, top: i32) -> Option<SwashImage> {
    if cw < 2 || ch < 2 || cw > MAX_CELL || ch > MAX_CELL {
        return None;
    }
    let mut m = Mask::new(cw, ch);
    if !arms::draw(&mut m, c)
        && !doubles::draw(&mut m, c)
        && !blocks::draw(&mut m, c)
        && !round::draw(&mut m, c)
        && !braille::draw(&mut m, c)
        && !marks::draw(&mut m, c)
        && !strokes::draw(&mut m, c)
        && !sextants::draw(&mut m, c)
    {
        return None;
    }
    // A claimed character that rounds away to nothing at this cell size is
    // better left to the font than drawn as a blank. U+2800 is the one
    // character here that is GENUINELY blank — an empty braille pattern is a
    // real character crew's own spinner passes through — so it is exempt.
    if !m.inked() && c != '\u{2800}' {
        return None;
    }
    let mut image = SwashImage::new();
    image.placement = Placement {
        left: 0,
        top,
        width: cw,
        height: ch,
    };
    image.data = m.data;
    Some(image)
}

#[cfg(test)]
#[path = "boxglyph_tests.rs"]
mod tests;
