//! A fenced block's language, as a segment: `▐ rust ▌` on the language's
//! hue, laid on the code field where the muted label used to sit.
//!
//! `md::layout` builds the header line as text — `<icon> <label>` on a
//! Nerd Font, the bare label otherwise (`glyphs::fence_header`) — and the
//! chat card turns that text into a badge here. The language is read back
//! off the label rather than carried beside it: the label's first word
//! after any icon is exactly the key the icon was chosen by.
use crate::chatbody::CardCell;
use crate::segment::{self, Caps};

/// The badge cells for a `CodeHeader` line's `label`. Its block is the
/// language's hue ([`crate::chathue::lang_hue`]), its ink the page colour
/// walked to the text floor, its caps on the code field the row is laid
/// into.
pub(crate) fn cells(label: &str) -> Vec<CardCell> {
    let bg = crate::chathue::lang_hue(lang_of(label));
    let field = crate::chatink::code_bg();
    segment::to_card(&segment::badge(
        label,
        segment::page_ink(bg),
        bg,
        Caps::on(field),
    ))
}

/// The language word of a header label: the first word that is not an
/// icon. `code`, the untagged fence's label, keys to the default hue.
pub(crate) fn lang_of(label: &str) -> &str {
    label
        .split_whitespace()
        .find(|w| !w.chars().all(is_pua))
        .unwrap_or("")
}

/// Unicode Private Use — where every Nerd Font icon lives.
fn is_pua(c: char) -> bool {
    matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

#[cfg(test)]
#[path = "fencebadge_tests.rs"]
mod tests;
