//! A cell's resolved foreground and background, with reverse video (SGR 7)
//! honoured on the alternate screen only.
//!
//! On the primary screen agent CLIs use SGR 7 to "highlight" the line you just
//! sent, and the swap reads as a hard block over prose, so there it stays
//! plain. Full-screen programs on the alternate screen draw their bars with
//! it: vim's status line, nano's title and shortcut rows, less's `(END)` and
//! search hits, the selected row in mc, tig and htop. Without the swap those
//! were plain text with no bar at all.
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::Color;

use crate::color::{default_bg, default_fg, resolve_color};

/// `(fg, bg, swapped)` for one cell. `alt` is whether the alternate screen
/// is up.
///
/// The caller skips the echo-grey drop (`should_drop_bg`) on a swapped pair:
/// the bar's background is usually the default foreground, light grey on a
/// dark theme, which is exactly what that heuristic flattens, and the bar
/// would vanish again.
pub(crate) fn pair(
    fg: Color,
    bg: Color,
    flags: Flags,
    palette: &Colors,
    alt: bool,
) -> ((u8, u8, u8), (u8, u8, u8), bool) {
    let rfg = resolve_color(fg, palette, default_fg());
    let rbg = resolve_color(bg, palette, default_bg());
    match alt && flags.contains(Flags::INVERSE) {
        true => (rbg, rfg, true),
        false => (rfg, rbg, false),
    }
}

#[cfg(test)]
#[path = "modelinverse_tests.rs"]
mod tests;
