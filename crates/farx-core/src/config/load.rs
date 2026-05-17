use tracing::warn;

use super::paths;
use super::AppConfig;

impl AppConfig {
    /// Load configuration from `$CONFIG_DIR/farx/config.toml`.
    /// Falls back to defaults if the file does not exist or cannot be parsed.
    pub fn load() -> Self {
        let path = match paths::find_existing() {
            Some(p) => p,
            None => return Self::default(),
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<AppConfig>(&contents) {
                Ok(config) => {
                    eprintln!("[farx] Loaded config: theme={}", config.ui.theme);
                    config
                }
                Err(e) => {
                    eprintln!("[farx] Config parse error: {}", e);
                    warn!(
                        "Failed to parse config at {}: {}; using defaults",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read config at {}: {}; using defaults",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }
}
