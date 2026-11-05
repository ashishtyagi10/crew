//! The chips a monochrome palette shows in a rotation's strip.
//!
//! A palette is told apart by its accent on its page, and an Ink page has no
//! accent to speak of: `dark` writes near-white on black, `light` dark grey on
//! white. Drawn as every other chip — the accent square in a capsule of its
//! page — the one that does not match the page under the picker became a
//! white square ringed in black on the light page, and a dark one ringed in
//! white on the dark page: a hollow box, the very glyph the settings form
//! uses for an unticked option (UI survey loop 39, #9). What those palettes
//! ARE is a page and the words on it, so that is what they show: two bare
//! squares, page then ink, and either one that would vanish into the page
//! they are drawn on is walked until it reads as a mark.
use super::{Chip, FAINT, GREY};
use crew_theme::ThemeId;

/// `id`'s page and ink as two bare squares, or `None` when the palette has a
/// hue of its own and its accent chip already says which one it is.
pub(super) fn chips(id: ThemeId) -> Option<Vec<Chip>> {
    let t = id.theme();
    if !(grey(t.accent_default) && grey(t.ink)) {
        return None;
    }
    let page = crew_theme::theme().page_bg;
    let shown = |face| match crew_theme::contrast_ratio(face, page) < FAINT {
        true => crew_theme::readable::enforced(face, page, MARK),
        false => face,
    };
    let square = |face| Chip {
        c: '\u{25a0}',
        fg: shown(face),
        bg: None,
    };
    Some(vec![square(t.page_bg), square(t.ink)])
}

/// What a walked square is held to: the floor crew holds marks and non-text
/// signals to. Its own page drawn on itself is the square that walks the
/// furthest, and at the glass chips' 2:1 it was a smudge.
const MARK: f32 = 3.0;

/// A colour with no hue: its channels sit within [`GREY`] of each other.
fn grey(c: (u8, u8, u8)) -> bool {
    c.0.max(c.1).max(c.2) - c.0.min(c.1).min(c.2) < GREY
}

#[cfg(test)]
#[path = "swatchmono_tests.rs"]
mod tests;
