//! The SGR a DECRQSS `m` reports: the attributes and colours new text is
//! written in now (alacritty's cursor template), as the parameters that
//! would set them from a reset — `0;1;38;5;208;48;2;1;2;3`.
//!
//! The background comes last: nvim reads the last three parameters as the
//! `r;g;b` it set, so a reply that ended in anything else would read as "no
//! 24-bit colour" even when the colour was kept exactly.
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::vte::ansi::Color;

/// `0`, then one parameter (or group) per attribute and colour set.
pub(super) fn sgr(c: &Cell) -> String {
    let mut p: Vec<String> = vec!["0".into()];
    for (flag, code) in [
        (Flags::BOLD, "1"),
        (Flags::DIM, "2"),
        (Flags::ITALIC, "3"),
        (Flags::UNDERLINE, "4"),
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
    if let Some(u) = c.underline_color() {
        p.extend(color(u, 58, None));
    }
    p.extend(color(c.fg, 38, Some(30)));
    p.extend(color(c.bg, 48, Some(40)));
    p.join(";")
}

/// A colour as SGR: named slots as `30`–`37`/`90`–`97` (`base` 30 or 40),
/// indexed as `38;5;n`, direct as `38;2;r;g;b`; the default (or, for an
/// underline, any named slot) as nothing.
fn color(c: Color, extended: u8, base: Option<u8>) -> Option<String> {
    match c {
        Color::Spec(rgb) => Some(format!("{extended};2;{};{};{}", rgb.r, rgb.g, rgb.b)),
        Color::Indexed(i) => Some(format!("{extended};5;{i}")),
        Color::Named(n) => {
            let (i, base) = (n as usize, base?);
            match i {
                0..=7 => Some(format!("{}", base as usize + i)),
                8..=15 => Some(format!("{}", base as usize + 60 + i - 8)),
                // Foreground/Background: the default, which `0` already set.
                _ => None,
            }
        }
    }
}
