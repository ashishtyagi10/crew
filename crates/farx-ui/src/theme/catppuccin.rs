use ratatui::style::{Color, Modifier, Style};

use super::Theme;

pub(super) fn build() -> Theme {
    let base = Color::Rgb(30, 30, 46);
    let base_alt = Color::Rgb(35, 35, 52);
    let surface0 = Color::Rgb(49, 50, 68);
    let overlay0 = Color::Rgb(108, 112, 134);
    let text = Color::Rgb(205, 214, 244);
    let blue = Color::Rgb(137, 180, 250);
    let green = Color::Rgb(166, 227, 161);
    let mauve = Color::Rgb(203, 166, 247);
    let peach = Color::Rgb(250, 179, 135);
    let yellow = Color::Rgb(249, 226, 175);
    let teal = Color::Rgb(148, 226, 213);
    let _pink = Color::Rgb(245, 194, 231);
    let _red = Color::Rgb(243, 139, 168);

    Theme {
        name: "catppuccin",
        panel_bg: base,
        panel_bg_alt: base_alt,
        panel_fg: text,
        panel_header_fg: blue,
        column_header: Style::default()
            .fg(overlay0)
            .bg(surface0)
            .add_modifier(Modifier::BOLD),
        grid_separator: "│",
        grid_style: Style::default().fg(surface0).bg(base),
        panel_cursor: Style::default().fg(base).bg(blue),
        panel_selected: Style::default()
            .fg(yellow)
            .bg(Color::Rgb(55, 55, 75))
            .add_modifier(Modifier::BOLD),
        panel_cursor_selected: Style::default()
            .fg(yellow)
            .bg(blue)
            .add_modifier(Modifier::BOLD),
        panel_dir: Style::default()
            .fg(blue)
            .bg(base)
            .add_modifier(Modifier::BOLD),
        panel_exe: Style::default().fg(green).bg(base),
        panel_archive: Style::default().fg(mauve).bg(base),
        panel_symlink: Style::default()
            .fg(teal)
            .bg(base)
            .add_modifier(Modifier::ITALIC),
        panel_hidden: Style::default().fg(overlay0).bg(base),
        panel_image: Style::default().fg(peach).bg(base),
        panel_border: Style::default().fg(surface0).bg(base),
        panel_border_active: Style::default().fg(blue).bg(base),
        cmd_line: Style::default().fg(overlay0).bg(base),
        info_text: Style::default().fg(text).bg(base),
        footer: Style::default().fg(overlay0).bg(base),
    }
}
