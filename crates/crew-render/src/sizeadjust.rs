//! One apparent size whichever face draws the text — CSS `font-size-adjust`
//! for a cell grid.
//!
//! Every face is shaped at the same em size into the same 0.6-em cell, but
//! faces do not fill an em alike. Measured on the faces crew rotates through
//! (CoreText, 2026-11-15): the x-height runs from 0.45 em (Comic Shanns Mono)
//! to 0.55 (JetBrains Mono, MonoLisa), and the advance from 0.545 em (Comic
//! Mono) to 0.64 (MonoLisa). So a theme step that changed the face also
//! changed how big the text looked — Comic Shanns read a fifth smaller than
//! JetBrains, which is why Serious Shanns was asked out in a day — and the
//! faces wider than the cell (MonoLisa, SF Mono, Fira Code) were drawn into
//! it with their letters touching.
//!
//! [`scale`] answers both with one factor on the size a face is shaped at,
//! never on the cell: a face whose lower case is short grows toward
//! [`TARGET_X_HEIGHT`], and no face is drawn wider than the cell it sits in.
//! A face that already fits and reads at full size keeps 1.0 exactly, so the
//! faces crew was calibrated on (JetBrains Mono, Lilex, IBM Plex) do not move.
use glyphon::cosmic_text::fontdb;
use glyphon::{Family, FontSystem};

use crate::celltext::CELL_W_RATIO;

/// The lower-case height every face is brought up to, as a share of the em.
/// The middle of the faces the user named as favourites (0.516 IBM Plex and
/// Lilex, 0.529 SF Mono, 0.55 JetBrains Mono): short faces come up to the
/// pack, tall ones are not pulled down to it.
pub(crate) const TARGET_X_HEIGHT: f32 = 0.53;

/// The share of the cell a glyph's advance may fill. A whole cell: the
/// faces drawn at 0.6 em into a 0.6-em cell (most of them) are the reference
/// for how close letters sit.
const FIT: f32 = 1.0;

/// The factor `family` is shaped at. 1.0 for an unknown family, a face with
/// no x-height in its tables, or one that already fits and reads at size.
/// Measured against the cell's DESIGN width, [`CELL_W_RATIO`], not the
/// rounded pixel one: a face drawn at exactly 0.6 em keeps exactly 1.0 at
/// every size instead of wobbling by a percent as the rounding goes.
pub(crate) fn scale(fs: &mut FontSystem, family: Option<&str>, weight: u16) -> f32 {
    measure(fs, family, weight).map_or(1.0, |(xh, adv)| factor(xh, adv, CELL_W_RATIO))
}

/// The factor for a face with this x-height and advance (both in ems): grow
/// a short face toward [`TARGET_X_HEIGHT`], then hold any face to the cell.
pub(crate) fn factor(x_height: f32, advance: f32, cell_em: f32) -> f32 {
    if !(x_height > 0.0 && advance > 0.0 && cell_em > 0.0) {
        return 1.0;
    }
    let grow = (TARGET_X_HEIGHT / x_height).max(1.0);
    let fit = FIT * cell_em / advance;
    grow.min(fit)
}

/// `(x_height, advance of '0')` in ems for the face `family` resolves to.
fn measure(fs: &mut FontSystem, family: Option<&str>, weight: u16) -> Option<(f32, f32)> {
    let font = font(fs, family, weight)?;
    let face = font.as_swash();
    let m = face.metrics(&[]);
    let upem = f32::from(m.units_per_em);
    let gid = face.charmap().map('0');
    if upem <= 0.0 || gid == 0 {
        return None;
    }
    let advance = face.glyph_metrics(&[]).advance_width(gid);
    Some((m.x_height / upem, advance / upem))
}

/// The face `family` resolves to at `weight` (`None`/empty: the embedded one).
pub(crate) fn font(
    fs: &mut FontSystem,
    family: Option<&str>,
    weight: u16,
) -> Option<std::sync::Arc<glyphon::cosmic_text::Font>> {
    let family = match family {
        Some(name) if !name.is_empty() => Family::Name(name),
        _ => Family::Name(crew_theme::EMBEDDED_FAMILY),
    };
    let weight = fontdb::Weight(weight);
    let id = fs.db().query(&fontdb::Query {
        families: &[family],
        weight,
        ..Default::default()
    })?;
    fs.get_font(id, weight)
}

#[cfg(test)]
#[path = "sizeadjust_tests.rs"]
mod tests;
