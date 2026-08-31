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
