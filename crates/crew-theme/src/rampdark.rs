//! The text ladder's quiet rungs on a DARK page.
//!
//! [`crate::ramp`] spaces every role by WCAG ratio, and a ratio is blind to
//! polarity: 4.6:1 is APCA Lc 65 under dark ink on a light page and Lc 35
//! under light ink on a dark one. Measured over the shipped palettes
//! (2026-11-18), a dark page drew its unfocused legend at Lc 40–45, the input
//! bar's hint at 32–37 and the placeholder at 28–32, where every light page
//! drew the same roles at 70–78, 64–71 and 60–67: the dash's figures, a
//! pane's legend and the hint read as disabled on every dark palette.
//!
//! A dark page keeps its ladder's ink, body and border, and floors the quiet
//! rungs where APCA puts readable secondary text: the legend Lc ≈ 55, the
//! hints ≈ 46 (level with a dark page's comment, `commentread`), the
//! placeholder ≈ 41 (spot text, a step under them). On a dark page Lc runs
//! ≈ 7.6 × the WCAG ratio across this range, so the floors are written as the
//! ratios that reach them. The tubes already sit there (`HOUSE_CRT`) and
//! light pages above; both are untouched.
use crate::ramp::{House, HOUSE, HOUSE_INK};

/// The paper/modern ladder on a dark page.
pub const HOUSE_DARK: House = House {
    name: "paper/modern (dark)",
    legend_off: LEGEND_OFF,
    dim: DIM,
    hint_fg: HINT,
    placeholder: PLACEHOLDER,
    ..HOUSE
};

/// The Ink Reader ladder on a dark page (`paper-dark`).
pub const HOUSE_INK_DARK: House = House {
    name: "ink reader (print, dark)",
    legend_off: LEGEND_OFF,
    dim: DIM,
    hint_fg: HINT,
    placeholder: PLACEHOLDER,
    ..HOUSE_INK
};

const LEGEND_OFF: f32 = 7.2;
const DIM: f32 = 6.0;
const HINT: f32 = 6.1;
const PLACEHOLDER: f32 = 5.4;

/// `house` as it applies on a page that is `dark` or not: the paper and Ink
/// Reader ladders trade for their dark rungs, every other ladder holds.
pub fn for_page(house: House, dark: bool) -> House {
    match (dark, house.name) {
        (true, n) if n == HOUSE.name => HOUSE_DARK,
        (true, n) if n == HOUSE_INK.name => HOUSE_INK_DARK,
        _ => house,
    }
}

#[cfg(test)]
#[path = "rampdark_tests.rs"]
mod tests;
