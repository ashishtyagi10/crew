use std::path::PathBuf;

fn default_font_size() -> f32 {
    14.0
}

fn default_nav_width() -> f32 {
    210.0
}

fn default_show_nav() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_notify_min_secs() -> u64 {
    10
}

fn default_paper_grain() -> f32 {
    // ~2.6% luminance grain — clearly reads as paper texture without looking
    // noisy (chosen by comparing a rendered 0.0/0.6/1.0/1.6 sweep). Tunable in
    // config; 0.0 disables grain, paper_texture=false disables the whole pass.
    1.3
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CrewConfig {
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_nav_width")]
    pub nav_width: f32,
    #[serde(default = "default_show_nav")]
    pub show_nav: bool,
    /// Chosen font family; `None`/empty uses the system monospace.
    #[serde(default)]
    pub font_family: Option<String>,
    /// `/font random`: rotate the UI font every 10 minutes through the
    /// installed monospace families. The rotated pick itself is NOT saved —
    /// `font_family` stays whatever the user pinned.
    #[serde(default)]
    pub font_random: bool,
    /// Accent colour as a `#rrggbb` hex string; `None`/invalid uses the built-in
    /// Crew green. Applied app-wide via [`crate::palette`].
    #[serde(default)]
    pub accent: Option<String>,
    /// Whether the window should launch maximized.
    #[serde(default)]
    pub maximized: bool,
    /// Last working directory (absolute), restored on the next launch.
    #[serde(default)]
    pub last_dir: Option<String>,
    /// Last window size in logical pixels, restored on the next launch.
    #[serde(default)]
    pub win_w: Option<f32>,
    #[serde(default)]
    pub win_h: Option<f32>,
    /// Master switch for the notification system (pane events flashed on the
    /// input bar + logged in the sidebar). When off, no events are surfaced.
    #[serde(default = "default_true")]
    pub notify: bool,
    /// Notify when a foreground command in a pane finishes (returns to the shell
    /// prompt) after running at least `notify_min_secs`.
    #[serde(default = "default_true")]
    pub notify_agent_done: bool,
    /// Notify when a program rings the terminal bell.
    #[serde(default = "default_true")]
    pub notify_bell: bool,
    /// Notify when a pane's process exits.
    #[serde(default = "default_true")]
    pub notify_exit: bool,
    /// Minimum foreground-command runtime (seconds) before a "finished"
    /// notification fires — suppresses quick commands like `ls`/`cd`.
    #[serde(default = "default_notify_min_secs")]
    pub notify_min_secs: u64,
    /// Case-insensitive substrings watched in pane output; a match notifies.
    #[serde(default)]
    pub notify_patterns: Vec<String>,
    /// Theme name: `paper-dark` (default) or `paper-light`. Unknown/unset →
    /// `paper-dark`. Applied app-wide via [`crew_theme`].
    #[serde(default)]
    pub theme: Option<String>,
    /// Whether to render the subtle paper grain + vignette background texture.
    /// When off, the window background is a plain flat colour.
    #[serde(default = "default_true")]
    pub paper_texture: bool,
    /// Grain amplitude multiplier for the paper texture (0.0 = no grain, 1.0 = default ~3%, 2.0 = double).
    #[serde(default = "default_paper_grain")]
    pub paper_grain: f32,
    /// CRT tube post-process override. `None` (default) follows the active
    /// theme's `crt` flag — on for the `crt-*` phosphor themes, off elsewhere.
    /// `Some(true)`/`Some(false)` forces it via `/crt on|off` regardless of
    /// theme.
    #[serde(default)]
    pub crt: Option<bool>,
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            nav_width: default_nav_width(),
            show_nav: default_show_nav(),
            font_family: None,
            font_random: false,
            accent: None,
            maximized: false,
            last_dir: None,
            win_w: None,
            win_h: None,
            notify: true,
            notify_agent_done: true,
            notify_bell: true,
            notify_exit: true,
            notify_min_secs: default_notify_min_secs(),
            notify_patterns: Vec::new(),
            theme: None,
            paper_texture: true,
            paper_grain: default_paper_grain(),
            crt: None,
        }
    }
}

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
        }
    }

    pub fn from_toml_str(s: &str) -> Self {
        toml::from_str::<Self>(s).unwrap_or_default().clamped()
    }

    pub fn to_toml_str(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("crew").join("config.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => Self::from_toml_str(&contents),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        // Never write the real config from the test harness — the cwd tests
        // drive `set_cwd`, which would otherwise persist temp dirs into the
        // user's `last_dir` and reopen Crew in /tmp.
        if cfg!(test) {
            return;
        }
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&path, self.to_toml_str()) {
            eprintln!("crew: failed to save config: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CrewConfig;

    #[test]
    fn default_values() {
        let cfg = CrewConfig::default();
        assert_eq!(cfg.font_size, 14.0);
        assert!(cfg.show_nav);
    }

    #[test]
    fn notify_defaults_are_on() {
        let cfg = CrewConfig::default();
        assert!(cfg.notify);
        assert!(cfg.notify_agent_done);
        assert!(cfg.notify_bell);
        assert!(cfg.notify_exit);
        assert_eq!(cfg.notify_min_secs, 10);
        assert!(cfg.notify_patterns.is_empty());
    }

    #[test]
    fn notify_min_secs_clamped() {
        // Zero is nonsensical (every quick command fires) → clamp up to 1.
        let cfg = CrewConfig::from_toml_str("notify_min_secs = 0\n");
        assert_eq!(cfg.notify_min_secs, 1);
        // Absurdly large → clamped down to an hour.
        let cfg = CrewConfig::from_toml_str("notify_min_secs = 99999\n");
        assert_eq!(cfg.notify_min_secs, 3600);
    }

    #[test]
    fn notify_patterns_drop_blanks() {
        let cfg = CrewConfig::from_toml_str("notify_patterns = [\"error\", \"\", \"done\"]\n");
        assert_eq!(
            cfg.notify_patterns,
            vec!["error".to_string(), "done".to_string()]
        );
    }

    #[test]
    fn clamped_out_of_range() {
        let cfg = CrewConfig {
            font_size: 99.0,
            nav_width: 9.0,
            show_nav: true,
            font_family: None,
            accent: None,
            maximized: false,
            last_dir: None,
            win_w: Some(50.0),
            win_h: Some(50.0),
            ..CrewConfig::default()
        }
        .clamped();
        assert_eq!(cfg.font_size, 32.0);
        assert_eq!(cfg.nav_width, 160.0);
        assert!(cfg.show_nav);
        // window size is clamped up to sane minimums
        assert_eq!(cfg.win_w, Some(400.0));
        assert_eq!(cfg.win_h, Some(300.0));
    }

    #[test]
    fn from_toml_partial() {
        let cfg = CrewConfig::from_toml_str("font_size = 25.0\n");
        assert_eq!(cfg.font_size, 25.0);
        assert_eq!(cfg.nav_width, 210.0);
        assert!(cfg.show_nav);
    }

    #[test]
    fn from_toml_garbage() {
        let cfg = CrewConfig::from_toml_str("garbage {{{");
        assert_eq!(cfg, CrewConfig::default());
    }

    #[test]
    fn round_trip() {
        let c = CrewConfig {
            font_size: 20.0,
            nav_width: 200.0,
            show_nav: true,
            font_family: Some("Menlo".to_string()),
            font_random: false,
            accent: Some("#112233".to_string()),
            maximized: true,
            last_dir: Some("/tmp".to_string()),
            win_w: Some(1024.0),
            win_h: Some(768.0),
            notify: true,
            notify_agent_done: false,
            notify_bell: true,
            notify_exit: false,
            notify_min_secs: 30,
            notify_patterns: vec!["error".to_string(), "done".to_string()],
            theme: Some("paper-light".to_string()),
            paper_texture: false,
            paper_grain: 0.5,
            crt: Some(true),
        };
        assert_eq!(CrewConfig::from_toml_str(&c.to_toml_str()), c);
    }

    #[test]
    fn line_height() {
        let cfg = CrewConfig::default();
        assert!((cfg.line_height() - 17.5).abs() < 1e-6);
    }

    #[test]
    fn accent_rgb_parses_or_falls_back() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        // Unset → active theme default.
        assert_eq!(
            CrewConfig::default().accent_rgb(),
            crew_theme::PAPER_DARK.accent_default
        );
        // Valid hex → parsed.
        let cfg = CrewConfig::from_toml_str("accent = \"#102030\"\n");
        assert_eq!(cfg.accent_rgb(), (0x10, 0x20, 0x30));
        // Invalid hex → theme default (not a panic).
        let bad = CrewConfig::from_toml_str("accent = \"not-a-color\"\n");
        assert_eq!(bad.accent_rgb(), crew_theme::PAPER_DARK.accent_default);
    }

    #[test]
    fn empty_accent_clamped_to_none() {
        let cfg = CrewConfig::from_toml_str("accent = \"\"\n");
        assert_eq!(cfg.accent, None);
    }

    #[test]
    fn theme_id_parses_or_defaults() {
        assert_eq!(
            CrewConfig::default().theme_id(),
            crew_theme::ThemeId::PaperDark
        );
        let light = CrewConfig::from_toml_str("theme = \"paper-light\"\n");
        assert_eq!(light.theme_id(), crew_theme::ThemeId::PaperLight);
        let bad = CrewConfig::from_toml_str("theme = \"chartreuse\"\n");
        assert_eq!(bad.theme_id(), crew_theme::ThemeId::PaperDark);
    }

    #[test]
    fn font_random_round_trips_and_defaults_off() {
        let cfg = CrewConfig::from_toml_str("");
        assert!(!cfg.font_random);
        let cfg = CrewConfig::from_toml_str("font_random = true\n");
        assert!(cfg.font_random);
        assert!(cfg.clamped().font_random, "clamped() must carry the flag");
    }
}
