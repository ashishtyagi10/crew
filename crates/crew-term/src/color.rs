use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, Rgb};

/// The active theme's terminal default foreground.
pub(crate) fn default_fg() -> (u8, u8, u8) {
    crew_theme::theme().term_fg
}

/// The active theme's terminal default background.
pub(crate) fn default_bg() -> (u8, u8, u8) {
    crew_theme::theme().term_bg
}

/// Answer a program's color query (OSC 4/10/11/12) for palette slot `index`,
/// mirroring how [`resolve_color`] paints: theme ANSI for 0–15, the standard
/// xterm cube/greyscale for 16–255, and the theme terminal fg/bg/cursor for
/// the named slots. `None` for slots we don't model (no reply is sent).
pub(crate) fn query_color(index: usize) -> Option<Rgb> {
    let t = crew_theme::theme();
    let (r, g, b) = match index {
        0..=15 => t.ansi[index],
        16..=231 => {
            let i = index - 16;
            let level = |v: usize| if v == 0 { 0 } else { (55 + 40 * v) as u8 };
            (level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        232..=255 => {
            let g = (8 + 10 * (index - 232)) as u8;
            (g, g, g)
        }
        256 => t.term_fg, // NamedColor::Foreground
        257 => t.term_bg, // NamedColor::Background
        258 => t.term_fg, // NamedColor::Cursor
        _ => return None,
    };
    Some(Rgb { r, g, b })
}

pub(crate) fn resolve_color(color: Color, palette: &Colors, default: (u8, u8, u8)) -> (u8, u8, u8) {
    let ansi = &crew_theme::theme().ansi;
    match color {
        Color::Spec(Rgb { r, g, b }) => (r, g, b),
        Color::Named(named) => {
            let idx = named as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
        Color::Indexed(i) => {
            let idx = i as usize;
            if let Some(rgb) = palette[idx] {
                (rgb.r, rgb.g, rgb.b)
            } else if idx < 16 {
                ansi[idx]
            } else {
                default
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::term::color::Colors;
    use alacritty_terminal::vte::ansi::{Color, NamedColor};

    #[test]
    fn named_red_resolves_to_active_theme_ansi() {
        crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
        let palette = Colors::default(); // all slots unset → fall back to theme
        let got = resolve_color(
            Color::Named(NamedColor::Red),
            &palette,
            crew_theme::theme().term_fg,
        );
        assert_eq!(got, crew_theme::PAPER_LIGHT.ansi[1]);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    }
}
