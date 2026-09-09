//! The two marks built from a glyph AND text (see [`crate::glyphs`]): a
//! fence header's `<icon> label` and a footnote's `<icon>label` — each with
//! a plain form the pane drew before there was an icon set. Split from
//! `glyphs` so that file stays the two tables and the switch.
use crate::glyphs::{nerd, on, Glyph};

/// A fence header's label: `<icon> <label>` on a Nerd Font, the bare label
/// otherwise (`code` untagged). Laid into a [`crate::segment`] badge by the
/// card, so it is clipped to `width` less the badge's caps and pads.
pub(crate) fn fence_header(lang: &str, width: usize) -> String {
    let label = if lang.is_empty() { "code" } else { lang };
    let text = if on() {
        format!("{} {label}", nerd(Glyph::Lang(lang)))
    } else {
        label.to_string()
    };
    let chrome = crate::segment::width("", crate::segment::Caps::BOTH);
    crate::chatwidth::clip_w(&text, width.saturating_sub(chrome))
}

/// A `[^label]` reference's mark: `<icon>label` on a Nerd Font, `[label]`
/// otherwise.
pub(crate) fn footnote_mark(label: &str) -> String {
    if on() {
        format!("{}{label}", nerd(Glyph::Footnote))
    } else {
        format!("[{label}]")
    }
}
