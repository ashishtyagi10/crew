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
    /// Whether the window should launch maximized.
    #[serde(default)]
    pub maximized: bool,
    /// Last working directory (absolute), restored on the next launch.
    #[serde(default)]
    pub last_dir: Option<String>,
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            nav_width: default_nav_width(),
            show_nav: default_show_nav(),
            font_family: None,
            maximized: false,
            last_dir: None,
        }
    }
}

impl CrewConfig {
    pub fn line_height(&self) -> f32 {
        self.font_size * 1.25
    }

    pub fn clamped(self) -> Self {
        Self {
            font_size: self.font_size.clamp(12.0, 32.0),
            nav_width: self.nav_width.clamp(160.0, 320.0),
            show_nav: self.show_nav,
            font_family: self.font_family.filter(|n| !n.is_empty()),
            maximized: self.maximized,
            last_dir: self.last_dir,
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
    fn clamped_out_of_range() {
        let cfg = CrewConfig {
            font_size: 99.0,
            nav_width: 9.0,
            show_nav: true,
            font_family: None,
            maximized: false,
            last_dir: None,
        }
        .clamped();
        assert_eq!(cfg.font_size, 32.0);
        assert_eq!(cfg.nav_width, 160.0);
        assert!(cfg.show_nav);
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
            maximized: true,
            last_dir: Some("/tmp".to_string()),
        };
        assert_eq!(CrewConfig::from_toml_str(&c.to_toml_str()), c);
    }

    #[test]
    fn line_height() {
        let cfg = CrewConfig::default();
        assert!((cfg.line_height() - 17.5).abs() < 1e-6);
    }
}
