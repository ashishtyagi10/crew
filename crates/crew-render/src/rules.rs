//! Where a cell's underline and strikethrough sit: on the face's own
//! metrics, the way Ghostty, kitty and Alacritty place them.
//!
//! Crew drew the underline two rule-widths up from the cell's bottom edge
//! and the strike through the cell's middle, whatever the face and the
//! leading. Measured against eight faces' own tables (2026-11-15) at 14 pt
//! on Retina: at the 1.25 cell the underline was where most of them ask
//! (within a pixel) but 6.5 px under the baseline on the two Comic faces,
//! whose lines are short; every step of `/leading` let it fall further from
//! its word (3-4 px at `loose`); and the strike ran 2.5-4 px above the
//! middle of the lower case on every face, through the tops of `a` and `e`.
//!
//! Every font says where these go (`post.underlinePosition`,
//! `OS/2.yStrikeoutPosition`), and the baseline is where cosmic-text puts
//! it: the face's ascent plus descent centred in the line. So the rules are
//! measured from that baseline, in the thickness crew already draws, and
//! held inside the cell.
use std::cell::RefCell;

use glyphon::FontSystem;

use crate::celltext::FontParams;
use crate::deco::thickness;

/// The rules' tops, in px from the cell's top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rules {
    pub(crate) under: f32,
    pub(crate) strike: f32,
}

impl Rules {
    /// Where crew drew them before it read the face: the underline a gap
    /// of one rule above the cell's bottom, the strike mid-cell. What a cell
    /// gets when the face cannot be read.
    pub(crate) fn plain(cell_h: f32) -> Self {
        let t = thickness(cell_h);
        Rules {
            under: (cell_h - 2.0 * t).round(),
            strike: ((cell_h - t) * 0.5).round(),
        }
    }

    /// From a face's metrics in ems (`ascent` up, `descent` down, both
    /// positive; `under` and `strike` the decorations' tops above the
    /// baseline, negative below), at `size` px in a `cell_h` px line.
    pub(crate) fn from_face(f: Face, size: f32, cell_h: f32) -> Self {
        let t = thickness(cell_h);
        let plain = Self::plain(cell_h);
        let baseline = (cell_h - (f.ascent + f.descent) * size) / 2.0 + f.ascent * size;
        // One clear pixel under the baseline, and never lower than crew's
        // old place: that is the room a double or curly rule is drawn in.
        let under = (baseline - f.under * size)
            .round()
            .max(baseline.round() + 1.0);
        let strike = (baseline - f.strike * size - t / 2.0).round();
        Rules {
            under: under.min(plain.under),
            strike: strike.clamp(0.0, cell_h - t),
        }
    }
}

/// A face's vertical metrics, in ems.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Face {
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    pub(crate) under: f32,
    pub(crate) strike: f32,
}

/// Where an underline's top sits when the face does not say: 0.12 em
/// under the baseline, the middle of what the faces crew rotates through
/// ask for (0.05 Fira Code to 0.155 JetBrains Mono).
const FALLBACK_UNDER: f32 = -0.12;

thread_local! {
    /// The last answer and what it was asked: one face per frame, so one
    /// entry is the whole working set.
    static LAST: RefCell<Option<((Option<String>, u16, u32, u32), Rules)>> = const { RefCell::new(None) };
}

/// The rules for text shaped with `params` in a `cell_h` px cell.
pub(crate) fn for_params(fs: &mut FontSystem, params: &FontParams, cell_h: f32) -> Rules {
    let key = (
        params.family.clone(),
        params.weight,
        params.font_size.to_bits(),
        cell_h.to_bits(),
    );
    if let Some(hit) = LAST.with_borrow(|l| l.as_ref().filter(|(k, _)| *k == key).map(|e| e.1)) {
        return hit;
    }
    let rules = face(fs, params.family.as_deref(), params.weight)
        .map_or(Rules::plain(cell_h), |f| {
            Rules::from_face(f, params.font_size, cell_h)
        });
    LAST.with_borrow_mut(|l| *l = Some((key, rules)));
    rules
}

/// [`Face`] for the face `family` resolves to, as cosmic-text reads it.
pub(crate) fn face(fs: &mut FontSystem, family: Option<&str>, weight: u16) -> Option<Face> {
    let font = crate::sizeadjust::font(fs, family, weight)?;
    let m = font.metrics();
    let upem = f32::from(m.units_per_em);
    if upem <= 0.0 {
        return None;
    }
    let ascent = m.ascent / upem;
    let descent = -m.descent / upem;
    let x_height = m.x_height.unwrap_or(0.5 * m.ascent) / upem;
    // The middle of the lower case is where a strike belongs; a face's own
    // figure is taken while it lands in the middle half of it (Comic Mono's
    // sits at a quarter, its foot).
    let mid = |d: f32| (0.25 * x_height..=0.75 * x_height).contains(&d);
    let strike = m
        .strikeout
        .map(|d| (d.offset - d.thickness / 2.0) / upem)
        .filter(|d| mid(*d))
        .unwrap_or(x_height / 2.0);
    // An underline at or above the baseline is a table nobody filled in
    // (both Comic faces say 0): the common figure stands in for it.
    let under = m
        .underline
        .map(|d| d.offset / upem)
        .filter(|u| *u < -0.02)
        .unwrap_or(FALLBACK_UNDER);
    Some(Face {
        ascent,
        descent,
        under,
        strike,
    })
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
