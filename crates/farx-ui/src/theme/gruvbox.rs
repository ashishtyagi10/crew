use ratatui::style::{Color, Modifier, Style};

use super::Theme;

pub(super) fn build() -> Theme {
    let bg = Color::Rgb(40, 40, 40);
    let bg_alt = Color::Rgb(50, 48, 47);
    let fg = Color::Rgb(235, 219, 178);
    let gray = Color::Rgb(146, 131, 116);
    let _red = Color::Rgb(251, 73, 52);
    let green = Color::Rgb(184, 187, 38);
    let yellow = Color::Rgb(250, 189, 47);
    let blue = Color::Rgb(131, 165, 152);
    let purple = Color::Rgb(211, 134, 155);
    let aqua = Color::Rgb(142, 192, 124);
    let orange = Color::Rgb(254, 128, 25);
    let bg_highlight = Color::Rgb(60, 56, 54);

    Theme {
        name: "gruvbox",
        panel_bg: bg,
        panel_bg_alt: bg_alt,
        panel_fg: fg,
        panel_header_fg: yellow,
        column_header: Style::default()
            .fg(gray)
            .bg(bg_highlight)
            .add_modifier(Modifier::BOLD),
        grid_separator: "│",
        grid_style: Style::default().fg(bg_highlight).bg(bg),
        panel_cursor: Style::default().fg(bg).bg(yellow),
        panel_selected: Style::default()
            .fg(orange)
            .bg(Color::Rgb(70, 65, 55))
            .add_modifier(Modifier::BOLD),
        panel_cursor_selected: Style::default()
            .fg(orange)
            .bg(yellow)
            .add_modifier(Modifier::BOLD),
        panel_dir: Style::default()
            .fg(blue)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
        panel_exe: Style::default().fg(green).bg(bg),
        panel_archive: Style::default().fg(purple).bg(bg),
        panel_symlink: Style::default()
            .fg(aqua)
            .bg(bg)
            .add_modifier(Modifier::ITALIC),
        panel_hidden: Style::default().fg(gray).bg(bg),
        panel_image: Style::default().fg(orange).bg(bg),
        panel_border: Style::default().fg(bg_highlight).bg(bg),
        panel_border_active: Style::default().fg(yellow).bg(bg),
        cmd_line: Style::default().fg(gray).bg(bg),
        info_text: Style::default().fg(fg).bg(bg),
        footer: Style::default().fg(gray).bg(bg),
    }
}
