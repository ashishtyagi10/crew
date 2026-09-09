//! The oh-my-posh segment: a label on a solid colour block, capped at each
//! end so it reads as a badge and not as a highlighted word.
//!
//! One primitive for every badge the crew pane draws — a fence's language,
//! a card's sender, the footer's routing mode and working agents, an h1's
//! mark — so they all share one geometry (a pad cell inside each cap) and
//! one contract: the label's ink is walked to the text floor against the
//! block it sits on, whatever colour the caller asked for. A badge is
//! always readable; a caller cannot ship one that is not.
//!
//! The caps come from [`crate::glyphs`]' switch. On a Nerd Font they are
//! the powerline round arcs (U+E0B6 / U+E0B4), the "modern" oh-my-posh look.
//! Off it they are half blocks (`▐` / `▌`): a cap cell draws in the block's
//! colour on the ground behind it, so the half block IS the block's edge,
//! painted by ink rather than by a glyph a plain font might lack.
use crate::chatbody::{CardCell, Color};

/// Powerline round left cap (nf-ple-left_half_circle_thick).
pub(crate) const LEFT_CAP_NERD: char = '\u{e0b6}';
/// Powerline round right cap (nf-ple-right_half_circle_thick).
pub(crate) const RIGHT_CAP_NERD: char = '\u{e0b4}';
/// The right half block — the LEFT edge of a badge when the set is off.
pub(crate) const LEFT_CAP: char = '\u{2590}';
/// The left half block — the RIGHT edge of a badge when the set is off.
pub(crate) const RIGHT_CAP: char = '\u{258c}';

/// Cells of padding inside each cap.
pub(crate) const PAD: usize = 1;

/// Which ends of a badge are capped, and what lies behind the caps.
///
/// `ground` is the colour a cap cell's background takes: `None` is the page
/// (whatever the card or footer is drawn on — a wash included), `Some` is a
/// field the badge sits inside, like a fence's code tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Caps {
    pub left: bool,
    pub right: bool,
    pub ground: Option<Color>,
}

impl Caps {
    /// Both ends capped, on the page.
    pub(crate) const BOTH: Caps = Caps {
        left: true,
        right: true,
        ground: None,
    };

    /// Both ends capped, sitting on `ground`.
    pub(crate) fn on(ground: Color) -> Caps {
        Caps {
            ground: Some(ground),
            ..Caps::BOTH
        }
    }
}

/// One cell of a badge. `bg: None` is the page — only a cap cell has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    pub c: char,
    pub fg: Color,
    pub bg: Option<Color>,
    pub bold: bool,
}

/// The left cap glyph right now (see the module doc).
pub(crate) fn left_cap() -> char {
    if crate::glyphs::on() {
        LEFT_CAP_NERD
    } else {
        LEFT_CAP
    }
}

/// The right cap glyph right now.
pub(crate) fn right_cap() -> char {
    if crate::glyphs::on() {
        RIGHT_CAP_NERD
    } else {
        RIGHT_CAP
    }
}

/// Display columns a badge of `text` takes with `caps` — the label plus a
/// pad each side plus each cap. `width("", Caps::BOTH)` is the chrome
/// alone, which is what a caller clipping a label to fit subtracts.
pub(crate) fn width(text: &str, caps: Caps) -> usize {
    crate::chatwidth::str_w(text) + 2 * PAD + usize::from(caps.left) + usize::from(caps.right)
}

/// The ink a badge on `bg` reads in when its caller has no opinion: the
/// page's own colour, walked to the text floor. On a dark page that is dark
/// ink on a bright block, on a light page light ink on a deep one — the
/// oh-my-posh look on either.
pub(crate) fn page_ink(bg: Color) -> Color {
    crew_theme::readable::enforced(
        crew_theme::theme().page_bg,
        bg,
        crew_theme::contrast::text_floor(),
    )
}

/// `text` on a `bg` block, in `fg` walked to the text floor against `bg`
/// (`readable::enforced`, so it always clears), capped per `caps`. The
/// label is bold — a badge is a label, and a label carries weight.
pub(crate) fn badge(text: &str, fg: Color, bg: Color, caps: Caps) -> Vec<Cell> {
    let fg = crew_theme::readable::enforced(fg, bg, crew_theme::contrast::text_floor());
    let block = |c: char, bold: bool| Cell {
        c,
        fg,
        bg: Some(bg),
        bold,
    };
    let cap = |c: char| Cell {
        c,
        fg: bg,
        bg: caps.ground,
        bold: false,
    };
    let mut out = Vec::with_capacity(width(text, caps));
    if caps.left {
        out.push(cap(left_cap()));
    }
    out.extend(std::iter::repeat_n(block(' ', false), PAD));
    out.extend(text.chars().map(|c| block(c, true)));
    out.extend(std::iter::repeat_n(block(' ', false), PAD));
    if caps.right {
        out.push(cap(right_cap()));
    }
    out
}

/// A badge as card cells, for the chat transcript. The cells are
/// renderer-added chrome (`src: None`): a badge's characters came from no
/// byte of the message.
pub(crate) fn to_card(cells: &[Cell]) -> Vec<CardCell> {
    cells
        .iter()
        .map(|s| CardCell {
            bg: s.bg,
            ..crate::chatbody::plain(s.c, s.fg, s.bold)
        })
        .collect()
}

#[cfg(test)]
#[path = "segment_tests.rs"]
mod tests;
