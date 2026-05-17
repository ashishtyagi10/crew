use ratatui::style::{Color, Modifier, Style};

use super::Theme;

pub(super) fn build() -> Theme {
    let panel_bg = Color::Indexed(18);
    let panel_fg = Color::Cyan;

    Theme {
        name: "far-classic",
        panel_bg,
        panel_bg_alt: Color::Indexed(19),
        panel_fg,
        panel_header_fg: Color::Yellow,
        column_header: Style::default()
            .fg(Color::Yellow)
            .bg(panel_bg)
            .add_modifier(Modifier::BOLD),
        grid_separator: "│",
        grid_style: Style::default().fg(Color::Indexed(24)).bg(panel_bg),
        panel_cursor: Style::default().fg(Color::Black).bg(Color::Indexed(30)),
        panel_selected: Style::default()
            .fg(Color::Yellow)
            .bg(Color::Indexed(24))
            .add_modifier(Modifier::BOLD),
        panel_cursor_selected: Style::default()
            .fg(Color::Yellow)
            .bg(Color::Indexed(30))
            .add_modifier(Modifier::BOLD),
        panel_dir: Style::default()
            .fg(Color::White)
            .bg(panel_bg)
            .add_modifier(Modifier::BOLD),
        panel_exe: Style::default().fg(Color::Green).bg(panel_bg),
        panel_archive: Style::default().fg(Color::Magenta).bg(panel_bg),
        panel_symlink: Style::default().fg(Color::Cyan).bg(panel_bg),
        panel_hidden: Style::default().fg(Color::Indexed(244)).bg(panel_bg),
        panel_image: Style::default().fg(Color::Rgb(255, 150, 50)).bg(panel_bg),
        panel_border: Style::default().fg(Color::Indexed(24)).bg(panel_bg),
        panel_border_active: Style::default().fg(Color::Cyan).bg(panel_bg),
        fn_bar_bg: Color::Black,
        fn_bar_fg: Color::Cyan,
        fn_bar_key: Style::default().fg(Color::Black).bg(Color::Cyan),
        fn_bar_label: Style::default().fg(Color::Cyan).bg(Color::Black),
        cmd_line: Style::default().fg(Color::Gray).bg(Color::Black),
        info_text: Style::default().fg(Color::Cyan).bg(panel_bg),
        footer: Style::default().fg(Color::Yellow).bg(panel_bg),
    }
}
