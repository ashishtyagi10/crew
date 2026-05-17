use ratatui::style::{Color, Modifier, Style};

use super::Theme;

pub(super) fn build() -> Theme {
    let bg = Color::Rgb(16, 16, 18); // near-black
    let bg_alt = Color::Rgb(22, 22, 25); // subtle stripe
    let fg = Color::Rgb(190, 186, 178); // warm gray text
    let accent = Color::Rgb(220, 170, 60); // warm amber/gold
    let green = Color::Rgb(120, 190, 90); // muted green
    let magenta = Color::Rgb(190, 120, 170); // dusty pink
    let orange = Color::Rgb(230, 140, 70); // warm orange
    let teal = Color::Rgb(90, 180, 160); // muted teal (not blue)
    let _yellow = Color::Rgb(230, 200, 100); // soft yellow
    let dim = Color::Rgb(70, 68, 64); // muted comments
    let surface = Color::Rgb(26, 26, 30); // surface for headers
    let cursor_bg = Color::Rgb(55, 50, 35); // warm dark highlight

    Theme {
        name: "tokyo-night",
        panel_bg: bg,
        panel_bg_alt: bg_alt,
        panel_fg: fg,
        panel_header_fg: accent,
        column_header: Style::default()
            .fg(Color::Rgb(120, 115, 105))
            .bg(surface)
            .add_modifier(Modifier::BOLD),
        grid_separator: "│",
        grid_style: Style::default().fg(Color::Rgb(40, 40, 42)).bg(bg),
        panel_cursor: Style::default().fg(Color::Rgb(240, 235, 220)).bg(cursor_bg),
        panel_selected: Style::default()
            .fg(Color::Rgb(255, 220, 80))
            .bg(Color::Rgb(50, 45, 25))
            .add_modifier(Modifier::BOLD),
        panel_cursor_selected: Style::default()
            .fg(Color::Rgb(255, 220, 80))
            .bg(cursor_bg)
            .add_modifier(Modifier::BOLD),
        panel_dir: Style::default()
            .fg(accent)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
        panel_exe: Style::default().fg(green).bg(bg),
        panel_archive: Style::default().fg(magenta).bg(bg),
        panel_symlink: Style::default()
            .fg(teal)
            .bg(bg)
            .add_modifier(Modifier::ITALIC),
        panel_hidden: Style::default().fg(dim).bg(bg),
        panel_image: Style::default().fg(orange).bg(bg),
        panel_border: Style::default().fg(Color::Rgb(40, 40, 42)).bg(bg),
        panel_border_active: Style::default().fg(accent).bg(bg),
        fn_bar_bg: surface,
        fn_bar_fg: fg,
        fn_bar_key: Style::default().fg(Color::Rgb(16, 16, 18)).bg(accent),
        fn_bar_label: Style::default().fg(fg).bg(surface),
        cmd_line: Style::default().fg(dim).bg(bg),
        info_text: Style::default().fg(fg).bg(bg),
        footer: Style::default().fg(dim).bg(bg),
    }
}
