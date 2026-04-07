use ratatui::style::{Color, Modifier, Style};

/// Visual theme definition for all farx UI elements.
pub struct Theme {
    pub name: &'static str,
    /// Panel background color.
    pub panel_bg: Color,
    /// Alternate row background (for grid/zebra striping).
    pub panel_bg_alt: Color,
    /// Default panel foreground (file text).
    pub panel_fg: Color,
    /// Panel header/title foreground.
    pub panel_header_fg: Color,
    /// Column header style.
    pub column_header: Style,
    /// Grid separator character and style.
    pub grid_separator: &'static str,
    pub grid_style: Style,
    /// Style for the cursor (highlighted) line.
    pub panel_cursor: Style,
    /// Style for selected file entries.
    pub panel_selected: Style,
    /// Style for cursor + selected.
    pub panel_cursor_selected: Style,
    /// Style for directory entries.
    pub panel_dir: Style,
    /// Style for executable files.
    pub panel_exe: Style,
    /// Style for archive files.
    pub panel_archive: Style,
    /// Style for symlinks.
    pub panel_symlink: Style,
    /// Style for hidden files.
    pub panel_hidden: Style,
    /// Style for image files.
    pub panel_image: Style,
    /// Style for panel borders.
    pub panel_border: Style,
    /// Active panel border.
    pub panel_border_active: Style,
    /// Function key bar background.
    pub fn_bar_bg: Color,
    /// Function key bar foreground.
    pub fn_bar_fg: Color,
    /// Style for the key number in the function bar.
    pub fn_bar_key: Style,
    /// Style for the label text in the function bar.
    pub fn_bar_label: Style,
    /// Style for the command line area.
    pub cmd_line: Style,
    /// Style for informational text.
    pub info_text: Style,
    /// Footer style.
    pub footer: Style,
}

impl Theme {
    /// Classic FAR Manager blue theme.
    pub fn far_classic() -> Self {
        let panel_bg = Color::Indexed(18);
        let panel_fg = Color::Cyan;

        Self {
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

    /// Modern dark theme — true black, warm amber/emerald accents, zero blue.
    pub fn tokyo_night() -> Self {
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

        Self {
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

    /// Catppuccin Mocha - warm dark theme.
    pub fn catppuccin() -> Self {
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

        Self {
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
            fn_bar_bg: surface0,
            fn_bar_fg: text,
            fn_bar_key: Style::default().fg(base).bg(blue),
            fn_bar_label: Style::default().fg(text).bg(surface0),
            cmd_line: Style::default().fg(overlay0).bg(base),
            info_text: Style::default().fg(text).bg(base),
            footer: Style::default().fg(overlay0).bg(base),
        }
    }

    /// Dracula theme.
    pub fn dracula() -> Self {
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

        Self {
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

    /// Gruvbox Dark theme.
    pub fn gruvbox() -> Self {
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

        Self {
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
            fn_bar_bg: bg_highlight,
            fn_bar_fg: fg,
            fn_bar_key: Style::default().fg(bg).bg(yellow),
            fn_bar_label: Style::default().fg(fg).bg(bg_highlight),
            cmd_line: Style::default().fg(gray).bg(bg),
            info_text: Style::default().fg(fg).bg(bg),
            footer: Style::default().fg(gray).bg(bg),
        }
    }

    /// Get theme by name.
    pub fn by_name(name: &str) -> Self {
        match name {
            "tokyo-night" => Self::tokyo_night(),
            "catppuccin" => Self::catppuccin(),
            "dracula" => Self::dracula(),
            "gruvbox" => Self::gruvbox(),
            _ => Self::far_classic(),
        }
    }

    /// List available theme names.
    pub fn available() -> &'static [&'static str] {
        &[
            "far-classic",
            "tokyo-night",
            "catppuccin",
            "dracula",
            "gruvbox",
        ]
    }
}
