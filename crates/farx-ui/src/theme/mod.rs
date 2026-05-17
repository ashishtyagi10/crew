use ratatui::style::{Color, Style};

mod catppuccin;
mod dracula;
mod far_classic;
mod gruvbox;
mod tokyo_night;

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
        far_classic::build()
    }

    /// Modern dark theme — true black, warm amber/emerald accents, zero blue.
    pub fn tokyo_night() -> Self {
        tokyo_night::build()
    }

    /// Catppuccin Mocha - warm dark theme.
    pub fn catppuccin() -> Self {
        catppuccin::build()
    }

    /// Dracula theme.
    pub fn dracula() -> Self {
        dracula::build()
    }

    /// Gruvbox Dark theme.
    pub fn gruvbox() -> Self {
        gruvbox::build()
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
