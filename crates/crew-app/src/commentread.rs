//! A comment on a dark page reads as a comment, not as a shadow of one.
//!
//! The syntax ladder sets a comment back from the ink by a luminance ratio
//! ([`crate::chatink::COMMENT_FLOOR`]), and a ratio is blind to polarity:
//! three steps under near-white ink is far dimmer to the eye than three
//! steps over near-black. Measured in APCA Lc on the code field
//! (2026-11-15): light pages drew comments at 56-61, every dark page at
//! 32-40 — paper-dark 32, neon 33, nebula and amethyst 35 — where APCA puts
//! spot text at 30. Editors that are praised for their dark comments sit
//! well clear of that (GitHub Dark's grey is about Lc 50).
//!
//! So on a dark page the comment is lifted, at its own hue and chroma, to
//! [`COMMENT_LC`], and never past the point where code stops standing a
//! rung ([`RUNG`]) above it: the ladder still says which is which. A light
//! page already clears the floor and a tube's ladder is its only
//! highlighting, so both keep what they had.
use crate::chatbody::Color;
use crew_theme::{apca, contrast_ratio, oklch, Theme};

/// The Lc a dark page's comment reads at on the code field: APCA's level
/// for secondary text that is still read, not merely noticed.
pub(crate) const COMMENT_LC: f32 = 45.0;

/// How far code stays above a comment, as a ratio: the glass's rung
/// (`codefield`), a step over the ladder's 1.15.
const RUNG: f32 = 1.2;

/// `comment` lifted to [`COMMENT_LC`] on `field`, stopping a [`RUNG`] under
/// `code`. Unchanged on a light page, on a tube, or where it already reads.
pub(crate) fn lift(comment: Color, t: &Theme, code: Color, field: Color) -> Color {
    if !t.dark || t.is_tube() {
        return comment;
    }
    let mut o = oklch::from_srgb(comment);
    let mut best = comment;
    while apca::lc(best, field) < COMMENT_LC && o.l < 1.0 {
        o.l += 0.004;
        let cand = o.to_srgb();
        if contrast_ratio(code, cand) < RUNG {
            break;
        }
        best = cand;
    }
    best
}

#[cfg(test)]
#[path = "commentread_tests.rs"]
mod tests;
