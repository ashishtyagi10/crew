//! `CrewConfig` persistence: TOML (de)serialisation and the on-disk
//! config path. Split from `config.rs` (child module).
use super::*;

impl CrewConfig {
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
