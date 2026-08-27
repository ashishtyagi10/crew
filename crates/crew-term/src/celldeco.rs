//! Reading a grid cell's decoration flags.
//!
//! The grid can have several underline bits set at once (a program that turns
//! on SGR 4 and then SGR 4:3 without resetting), so the mapping is a
//! precedence, not a match: the most specific underline a cell asked for is
//! the one it gets.
use alacritty_terminal::term::cell::Flags;
use crew_theme::deco::{Deco, DecoLine};

/// The underline this cell wears, most specific bit first.
pub(crate) fn line_of(flags: Flags) -> DecoLine {
    if flags.contains(Flags::UNDERCURL) {
        DecoLine::Curly
    } else if flags.contains(Flags::DOTTED_UNDERLINE) {
        DecoLine::Dotted
    } else if flags.contains(Flags::DASHED_UNDERLINE) {
        DecoLine::Dashed
    } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
        DecoLine::Double
    } else if flags.contains(Flags::UNDERLINE) {
        DecoLine::Single
    } else {
        DecoLine::None
    }
}

/// Whether this cell draws anything beyond its glyph. Blank cells are dropped
/// before they reach the renderer — but a *decorated* blank is a rule the
/// program asked for (the gap between two underlined words), so this decides
/// which spaces survive that filter.
pub(crate) fn decorated(flags: Flags) -> bool {
    line_of(flags) != DecoLine::None || flags.contains(Flags::STRIKEOUT)
}

/// The full decoration, with SGR 58's colour when the program set one.
pub(crate) fn deco_of(flags: Flags, color: Option<(u8, u8, u8)>) -> Deco {
    Deco {
        line: line_of(flags),
        strike: flags.contains(Flags::STRIKEOUT),
        color,
    }
}

#[cfg(test)]
#[path = "celldeco_tests.rs"]
mod celldeco_tests;
