//! Six colours that were picked against a dark page and never met a light one.
//!
//! ## What was wrong
//!
//! The ramp and the signal roles are derived and measured. These six were not
//! — they were constants sitting in whichever crate happened to draw them, and
//! every one of them was chosen by eye on a dark theme. Measured against the
//! page each actually lands on (WCAG contrast ratio):
//!
//! | role | dark pages | light pages |
//! |---|---|---|
//! | terminal cursor, focused | 11.2–12.3 | **1.41–1.61** |
//! | terminal cursor, unfocused | 2.7–3.0 | 5.7–6.6 |
//! | URL in a terminal | 7.7–8.4 | **2.05–2.35** |
//! | selection, against the text on it | 5.6–7.1 | **2.32–2.52** |
//! | load-average warning amber | 9.8–10.8 | **1.60–1.83** |
//! | load-average danger red | 5.3–5.9 | **2.95–3.39** |
//! | network sparkline | 10.2–11.3 | **1.53–1.76** |
//!
//! Three of those are worse than "a bit faint". The **cursor is inverted**: on
//! a light page the pane you are typing in has the faintest cursor on the
//! canvas, four times fainter than every pane you are not typing in. The
//! **warning amber is invisible** — a warning colour at 1.6 is a warning
//! nobody receives. And a **URL at 2.2** is a link you cannot read on the
//! third of the themes that ship light.
//!
//! ## The rule
//!
//! Each role keeps its **hue** — a link is blue, a warning is amber, an alarm
//! is red, and those meanings do not belong to the palette — and gives up its
//! **lightness**, which does. [`against`] walks the colour's L in Oklch toward
//! whichever pole the page is not, until it clears the floor its job demands.
//! On a dark page the intended colour usually already clears it and comes back
//! untouched; on a light page it darkens until it reads.
//!
//! Floors follow WCAG's two bands: 4.5 for anything that is text or carries
//! the cursor, 3.0 for a mark you only have to *see* (a sparkline, a dot).
//!
//! Every function here takes the `Theme` rather than reading the process
//! global, so the contract below can measure all nine palettes without nine
//! tests racing each other over one global.
use crate::oklch;
use crate::{contrast_ratio, Theme};

/// Text and text-like marks: the cursor block, a URL, a gauge readout.
///
/// The AA band, and the number every measurement in this module's contract is
/// written against. The floor actually applied is [`crate::contrast`]'s, which
/// is this one until the OS asks for more (see that module) — so a role is
/// derived against what the *user* asked for, not against a constant.
pub const TEXT_FLOOR: f32 = 4.5;
/// Marks you only have to see, not read: sparklines, dots, thumbs.
pub const MARK_FLOOR: f32 = 3.0;

/// How far one step moves the colour's L. Small enough that the result stops
/// as soon as it clears, rather than overshooting into a different colour.
const STEP: f32 = 0.02;

/// `want`, kept at its hue and chroma, walked toward the pole `page` is not
/// until it clears `floor` against `page`. Returns `want` unchanged when it
/// already clears — the common case on the pages these colours were chosen
/// on — and the best it reached when the hue simply cannot get there.
pub fn against(want: (u8, u8, u8), page: (u8, u8, u8), floor: f32) -> (u8, u8, u8) {
    if contrast_ratio(want, page) >= floor {
        return want;
    }
    let c = oklch::from_srgb(want);
    // Away from the page: a light page pushes the colour down, a dark one up.
    let dir = if oklch::from_srgb(page).l > c.l {
        -1.0
    } else {
        1.0
    };
    let mut best = want;
    let mut best_r = contrast_ratio(want, page);
    let mut l = c.l;
    for _ in 0..50 {
        l = (l + dir * STEP).clamp(0.0, 1.0);
        let rgb = c.with_l(l).to_srgb();
        let r = contrast_ratio(rgb, page);
        if r > best_r {
            (best, best_r) = (rgb, r);
        }
        if r >= floor {
            return rgb;
        }
        if l <= 0.0 || l >= 1.0 {
            break;
        }
    }
    best
}

/// One rank quieter than `want` on `page`: the same hue and chroma, pulled
/// part of the way toward the page's own lightness, then floored back to
/// [`MARK_FLOOR`] if the pull took it under.
///
/// For a reading that is the *same kind* of reading as the one beside it but
/// older or less important — the 5- and 15-minute load next to the 1-minute
/// one. Changing the hue instead would say the three measure different things,
/// and dropping to `text_muted` would throw away the warning colour they are
/// all carrying.
pub fn secondary(want: (u8, u8, u8), page: (u8, u8, u8)) -> (u8, u8, u8) {
    let (c, p) = (oklch::from_srgb(want), oklch::from_srgb(page));
    let pulled = c.with_l(c.l + (p.l - c.l) * 0.42).to_srgb();
    against(pulled, page, MARK_FLOOR)
}

/// The block cursor. Focused, it is the page's own ink inverted — the highest
/// contrast the palette has, in either direction. Unfocused it is the same
/// colour pulled most of the way back to the page: present, clearly secondary,
/// and never brighter than the focused one, which is the failure the constants
/// shipped with.
pub fn cursor(t: &Theme, focused: bool) -> (u8, u8, u8) {
    if focused {
        return against(t.ink, t.term_bg, crate::contrast::text_floor());
    }
    let ink = oklch::from_srgb(t.ink);
    let page = oklch::from_srgb(t.term_bg);
    ink.with_l(page.l + (ink.l - page.l) * 0.5).to_srgb()
}

/// The ink drawn *on* a cursor block or a selection: the page it replaced,
/// pushed until it reads against the block. A glyph under the cursor is still
/// a glyph.
pub fn on_block(t: &Theme, block: (u8, u8, u8)) -> (u8, u8, u8) {
    against(t.term_bg, block, crate::contrast::text_floor())
}

/// A clickable URL in terminal output. Blue is the convention and stays blue.
pub fn link(t: &Theme) -> (u8, u8, u8) {
    against(LINK_HUE, t.term_bg, crate::contrast::text_floor())
}

/// The mouse-selection wash behind terminal text.
pub fn selection_bg(t: &Theme) -> (u8, u8, u8) {
    against(SELECTION_HUE, t.term_fg, crate::contrast::text_floor())
}

/// A gauge crossing into "watch this" (load average, disk).
pub fn warn(t: &Theme) -> (u8, u8, u8) {
    against(WARN_HUE, t.page_bg, crate::contrast::text_floor())
}

/// A gauge past its limit.
pub fn danger(t: &Theme) -> (u8, u8, u8) {
    against(DANGER_HUE, t.page_bg, crate::contrast::text_floor())
}

/// A sparkline trace — seen, not read.
pub fn spark(t: &Theme) -> (u8, u8, u8) {
    against(SPARK_HUE, t.page_bg, crate::contrast::mark_floor())
}

/// The intended colours, which are now only intentions: each names a hue, and
/// the page it lands on decides how light it ends up.
const LINK_HUE: (u8, u8, u8) = (90, 170, 255);
const SELECTION_HUE: (u8, u8, u8) = (54, 84, 130);
const WARN_HUE: (u8, u8, u8) = (230, 180, 90);
const DANGER_HUE: (u8, u8, u8) = (230, 90, 90);
const SPARK_HUE: (u8, u8, u8) = (120, 200, 255);

#[cfg(test)]
#[path = "readable_tests.rs"]
mod tests;
