//! The SGR a DECRQSS `m` reports: the attributes and colours new text is
//! written in now (alacritty's cursor template), in xterm's own form
//! (`xtermFormatSGR`): `0`, then each attribute, then the colours — direct as
//! `38:2::r:g:b`, indexed from 16 up as `38:5:n`, the first sixteen as
//! `3x`/`9x` (`4x`/`10x` behind).
//!
//! The form is the contract: nvim's probe splits the reply on `;`, accepts
//! only `0` and one more parameter, and matches that one against
//! `48:2:`…; `0;48;2;1;2;3` reads as "no". (xterm and nvim source,
//! 2026-11-24.)
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::vte::ansi::Color;

/// `0`, then one parameter per attribute and colour set.
pub(super) fn sgr(c: &Cell) -> String {
    let mut p: Vec<String> = vec!["0".into()];
    for (flag, code) in [
        (Flags::BOLD, "1"),
        (Flags::DIM, "2"),
        (Flags::ITALIC, "3"),
        (Flags::UNDERLINE, "4"),
        // Not xterm's 21: crew's parser (vte) reads 21 as "bold off", so a
        // program that replays the reply to restore its state would lose the
        // underline. `4:n` is the spelling crew itself parses back.
        (Flags::DOUBLE_UNDERLINE, "4:2"),
        (Flags::UNDERCURL, "4:3"),
        (Flags::DOTTED_UNDERLINE, "4:4"),
        (Flags::DASHED_UNDERLINE, "4:5"),
        (Flags::INVERSE, "7"),
        (Flags::HIDDEN, "8"),
        (Flags::STRIKEOUT, "9"),
    ] {
        if c.flags.contains(flag) {
            p.push(code.into());
        }
    }
    p.extend(color(c.fg, 38, Some(30)));
    p.extend(color(c.bg, 48, Some(40)));
    if let Some(u) = c.underline_color() {
        p.extend(color(u, 58, None));
    }
    p.join(";")
}

/// A colour as one SGR parameter: `{38|48|58}:2::r:g:b` direct,
/// `{..}:5:n` indexed from 16, `3x`/`9x` (from `base`) for the first sixteen
/// whether set by name or by index, as xterm folds them; the default — or
/// an underline colour among the sixteen, which has no short form — as
/// `{58}:5:n` or nothing.
fn color(c: Color, extended: u8, base: Option<u8>) -> Option<String> {
    let index = match c {
        Color::Spec(rgb) => return Some(format!("{extended}:2::{}:{}:{}", rgb.r, rgb.g, rgb.b)),
        Color::Indexed(i) => usize::from(i),
        // Foreground/Background (256, 257): the default, which `0` set.
        Color::Named(n) if (n as usize) < 16 => n as usize,
        Color::Named(_) => return None,
    };
    match (index, base) {
        (0..=7, Some(b)) => Some(format!("{}", usize::from(b) + index)),
        (8..=15, Some(b)) => Some(format!("{}", usize::from(b) + 60 + index - 8)),
        _ => Some(format!("{extended}:5:{index}")),
    }
}
