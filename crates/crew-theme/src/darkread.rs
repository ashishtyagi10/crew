//! A coloured word on a dark page reads as brightly as on a light one.
//!
//! [`crate::readable`] floors its hues by WCAG ratio, which is blind to
//! polarity: on a dark page a hue clears the floor untouched while APCA,
//! which models how the eye reads light text on dark, puts it a band lower
//! than the same role on a light page. Measured 2026-11-18:
//!
//! - **The link** kept its shipped `(90, 170, 255)` at Lc 52.7–54.3 on
//!   paper-dark, neon, prism, slate and mercury, 57 on nebula and amethyst —
//!   under each palette's own ANSI blue (62–69) and every light page's link
//!   (63–71). The one word asking to be clicked was the dimmest coloured
//!   word on screen; under an unfocused pane's spotlight it fell to Lc 42.
//! - **Danger** (a LOAD figure past its limit, a failed swarm worker, a
//!   draft's tail) kept `(230, 90, 90)` at Lc 38–40 on the same five and
//!   51–55 on the rest, where light pages draw it at 61–70 and every page
//!   draws `warn`, the milder state, at 64–70: the alarm read quieter than
//!   the caution.
//!
//! Each is lifted at its own hue and chroma to [`READ_LC`] on a dark,
//! non-tube page. Light pages and the tubes (whose ladder is their only
//! highlighting) already sit at or above it, or by design below.
use crate::{apca, contrast_ratio, oklch, Theme};

/// Where APCA puts fluent body text's minimum: these words are read.
pub const READ_LC: f32 = 60.0;

/// `got` (a role floored from the hue `want`), or `want` lifted at its own
/// hue and chroma to [`READ_LC`] on `bg`, whichever reads more; `got` on a
/// light page or a tube. Lifted from the HUE, not from `got`, so a raised
/// contrast floor upstream can only move the answer further from the page.
pub fn lift(t: &Theme, want: Rgb, got: Rgb, bg: Rgb) -> Rgb {
    if !t.dark || t.is_tube() {
        return got;
    }
    let mut o = oklch::from_srgb(want);
    let mut best = want;
    while apca::lc(best, bg) < READ_LC && o.l < 0.97 {
        o.l += 0.002;
        best = o.to_srgb();
    }
    match contrast_ratio(best, bg) > contrast_ratio(got, bg) {
        true => best,
        false => got,
    }
}

type Rgb = (u8, u8, u8);

#[cfg(test)]
#[path = "darkread_tests.rs"]
mod tests;
