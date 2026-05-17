use ratatui::style::{Color, Modifier, Style};

use super::Theme;

pub(super) fn build() -> Theme {
    let bg = Color::Rgb(40, 42, 54);
    let bg_alt = Color::Rgb(46, 48, 62);
    let fg = Color::Rgb(248, 248, 242);
    let comment = Color::Rgb(98, 114, 164);
    let purple = Color::Rgb(189, 147, 249);
    let green = Color::Rgb(80, 250, 123);
    let pink = Color::Rgb(255, 121, 198);
    let cyan = Color::Rgb(139, 233, 253);
    let orange = Color::Rgb(255, 184, 108);
    let yellow = Color::Rgb(241, 250, 140);
    let current_line = Color::Rgb(68, 71, 90);

    Theme {
        name: "dracula",
        panel_bg: bg,
        panel_bg_alt: bg_alt,
        panel_fg: fg,
        panel_header_fg: purple,
        column_header: Style::default()
            .fg(comment)
            .bg(current_line)
            .add_modifier(Modifier::BOLD),
        grid_separator: "│",
        grid_style: Style::default().fg(current_line).bg(bg),
        panel_cursor: Style::default().fg(bg).bg(purple),
        panel_selected: Style::default()
            .fg(yellow)
            .bg(Color::Rgb(75, 78, 100))
            .add_modifier(Modifier::BOLD),
        panel_cursor_selected: Style::default()
            .fg(yellow)
            .bg(purple)
            .add_modifier(Modifier::BOLD),
        panel_dir: Style::default()
            .fg(purple)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
        panel_exe: Style::default().fg(green).bg(bg),
        panel_archive: Style::default().fg(pink).bg(bg),
        panel_symlink: Style::default()
            .fg(cyan)
            .bg(bg)
            .add_modifier(Modifier::ITALIC),
        panel_hidden: Style::default().fg(comment).bg(bg),
        panel_image: Style::default().fg(orange).bg(bg),
        panel_border: Style::default().fg(current_line).bg(bg),
        panel_border_active: Style::default().fg(purple).bg(bg),
        fn_bar_bg: current_line,
        fn_bar_fg: fg,
        fn_bar_key: Style::default().fg(bg).bg(green),
        fn_bar_label: Style::default().fg(fg).bg(current_line),
        cmd_line: Style::default().fg(comment).bg(bg),
        info_text: Style::default().fg(fg).bg(bg),
        footer: Style::default().fg(comment).bg(bg),
    }
}
