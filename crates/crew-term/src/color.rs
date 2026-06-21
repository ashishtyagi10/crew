use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, Rgb};

pub(crate) const DEFAULT_FG: (u8, u8, u8) = (0, 255, 160);
pub(crate) const DEFAULT_BG: (u8, u8, u8) = (8, 8, 16);

/// Standard xterm 16-color palette (indices 0–15) used when the terminal hasn't
/// explicitly defined those slots in its color table.
const ANSI16: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0  Black
    (170, 0, 0),     // 1  Red
    (0, 170, 0),     // 2  Green
    (170, 85, 0),    // 3  Yellow (dark)
    (0, 0, 170),     // 4  Blue
    (170, 0, 170),   // 5  Magenta
    (0, 170, 170),   // 6  Cyan
    (170, 170, 170), // 7  White
    (85, 85, 85),    // 8  Bright Black
    (255, 85, 85),   // 9  Bright Red
    (85, 255, 85),   // 10 Bright Green
    (255, 255, 85),  // 11 Bright Yellow
    (85, 85, 255),   // 12 Bright Blue
    (255, 85, 255),  // 13 Bright Magenta
    (85, 255, 255),  // 14 Bright Cyan
    (255, 255, 255), // 15 Bright White
];

pub(crate) fn resolve_color(color: Color, palette: &Colors, default: (u8, u8, u8)) -> (u8, u8, u8) {
    match color {
        Color::Spec(Rgb { r, g, b }) => (r, g, b),
        Color::Named(named) => {
            let idx = named as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ANSI16[idx]
            } else {
                default
            }
        }
        Color::Indexed(i) => {
            let idx = i as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ANSI16[idx]
            } else {
                default
            }
        }
    }
}
