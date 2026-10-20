//! The ink for a reply's strongest words: an h1 title and a table's column
//! names.
//!
//! It is the accent. On a hued palette the accent reads as "label" even below
//! the body's contrast, because the hue says so. A grey accent has no hue to
//! say it with, and a lower contrast then says only "less": on the Ink Reader
//! light page the h1 and the column names printed at 7.1:1 under 16.3:1 body
//! text and 11.2:1 rules — the title was the faintest heading in the reply,
//! and the header the faintest row of its table. With a grey accent the
//! strongest words take whichever of the accent, the ink and the focused
//! frame's ink reads hardest against the page.
use crate::chatbody::Color;

/// OKLCH chroma under which an accent is a grey: the paper presets' accents
/// sit near 0, and the least saturated hued accent is far above this.
const GREY_CHROMA: f32 = 0.02;

/// The h1 and table-header ink for the active theme and accent.
pub(crate) fn ink() -> Color {
    let accent = crate::palette::accent();
    if crew_theme::oklch::from_srgb(accent).c >= GREY_CHROMA {
        return accent;
    }
    let th = crew_theme::theme();
    let read = |c: Color| crew_theme::contrast_ratio(c, th.page_bg);
    [accent, th.ink, th.border_focused]
        .into_iter()
        .max_by(|a, b| read(*a).total_cmp(&read(*b)))
        .unwrap_or(accent)
}

#[cfg(test)]
#[path = "chatstrong_tests.rs"]
mod tests;
