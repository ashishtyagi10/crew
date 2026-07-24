//! Derived views over `CrewConfig`: theme/accent resolution, line height,
//! and range clamping. Split from `config.rs` (child module).
use super::*;

impl CrewConfig {
    pub fn line_height(&self) -> f32 {
        self.font_size * 1.25
    }

    /// The configured theme, or `paper-dark` when unset/unknown.
    pub fn theme_id(&self) -> crew_theme::ThemeId {
        self.theme
            .as_deref()
            .and_then(crew_theme::ThemeId::from_name)
            .unwrap_or(crew_theme::ThemeId::PaperDark)
    }

    /// A display label for the configured selection: the rotation mode name
    /// (`dark`/`light`/`crt`/`auto`) if it is one, the pinned palette name if
    /// a specific palette is saved, or `dark` when unset. Used by the settings
    /// picker, which now offers only the consolidated modes.
    pub fn theme_label(&self) -> String {
        match self.theme.as_deref().and_then(crew_theme::parse_selection) {
            Some(crew_theme::Selection::Mode(m)) => m.as_str().to_string(),
            Some(crew_theme::Selection::Fixed(id)) => id.as_str().to_string(),
            None => crew_theme::RandomMode::Dark.as_str().to_string(),
        }
    }

    /// The configured accent colour, or the active theme's default when unset/invalid.
    pub fn accent_rgb(&self) -> (u8, u8, u8) {
        self.accent
            .as_deref()
            .and_then(crate::palette::parse_hex)
            .unwrap_or_else(|| crew_theme::theme().accent_default)
    }

    pub fn clamped(self) -> Self {
        Self {
            font_size: self.font_size.clamp(12.0, 32.0),
            nav_width: self.nav_width.clamp(160.0, 320.0),
            show_nav: self.show_nav,
            font_family: self.font_family.filter(|n| !n.is_empty()),
            font_random: self.font_random,
            accent: self.accent.filter(|s| !s.is_empty()),
            maximized: self.maximized,
            last_dir: self.last_dir,
            win_w: self.win_w.map(|w| w.clamp(400.0, 10000.0)),
            win_h: self.win_h.map(|h| h.clamp(300.0, 10000.0)),
            notify: self.notify,
            notify_agent_done: self.notify_agent_done,
            notify_bell: self.notify_bell,
            notify_exit: self.notify_exit,
            notify_min_secs: self.notify_min_secs.clamp(1, 3600),
            notify_patterns: self
                .notify_patterns
                .into_iter()
                .filter(|p| !p.is_empty())
                .collect(),
            theme: self.theme.filter(|s| !s.is_empty()),
            paper_texture: self.paper_texture,
            paper_grain: self.paper_grain.clamp(0.0, 2.0),
            crt: self.crt,
            font_weight: self.font_weight.clamp(300, 900),
            usage_budget_5h: self.usage_budget_5h.max(10_000),
            usage_budget_7d: self.usage_budget_7d.max(10_000),
        }
    }
}
