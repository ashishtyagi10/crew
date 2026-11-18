//! A link on a dark page reads as brightly as the blue a program prints.
//!
//! [`crate::readable::link`] floors the link blue by WCAG ratio, and on a
//! dark page the shipped hue `(90, 170, 255)` clears that floor untouched, at
//! APCA Lc 53–54 on paper-dark, neon, prism, slate and mercury (2026-11-18)
//! — under every one of those palettes' own ANSI blue (Lc 62–69) and every
//! light page's link (63–71). A link is the one word in the output that asks
//! to be clicked, and it was the dimmest coloured word on the screen; under
//! an unfocused pane's spotlight it fell to Lc 42. Each light page and tube
//! already sits above [`LINK_LC`], so this moves only the dark pages.
use crate::{apca, oklch, Theme};

/// Where APCA puts fluent body text's minimum: a link is read, not glanced.
pub const LINK_LC: f32 = 60.0;

/// `link` lifted, at its own hue and chroma, to [`LINK_LC`] on `t`'s
/// terminal page; unchanged on a light page, a tube, or where it reads.
pub fn lift(t: &Theme, link: (u8, u8, u8)) -> (u8, u8, u8) {
    if !t.dark || t.is_tube() {
        return link;
    }
    let mut o = oklch::from_srgb(link);
    let mut best = link;
    while apca::lc(best, t.term_bg) < LINK_LC && o.l < 0.97 {
        o.l += 0.004;
        best = o.to_srgb();
    }
    best
}

#[cfg(test)]
#[path = "linklift_tests.rs"]
mod tests;
