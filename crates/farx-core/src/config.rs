use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub ui: UiConfig,
    pub panels: PanelConfig,
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
    pub show_hidden_files: bool,
    pub use_trash: bool,
    pub editor: String,
    pub viewer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub tick_rate_ms: u64,
    pub show_fn_bar: bool,
    pub date_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    pub directories_first: bool,
    pub default_sort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    /// "anthropic", "openrouter", or "openai-compatible"
    pub provider: String,
    /// Base URL for the API (e.g. "https://openrouter.ai/api/v1")
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    /// Environment variable name to read the API key from
    pub api_key_env: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            confirm_delete: true,
            confirm_overwrite: true,
            show_hidden_files: false,
            use_trash: true,
            editor: "internal".to_string(),
            viewer: "internal".to_string(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "tokyo-night".to_string(),
            tick_rate_ms: 250,
            show_fn_bar: true,
            date_format: "%Y-%m-%d %H:%M".to_string(),
        }
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            directories_first: true,
            default_sort: "name".to_string(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: "google/gemma-3-4b-it:free".to_string(),
            max_tokens: 4096,
            api_key_env: "OPENROUTER_API_KEY".to_string(),
        }
    }
}

impl AppConfig {
    /// Load configuration from `$CONFIG_DIR/farx/config.toml`.
    /// Falls back to defaults if the file does not exist or cannot be parsed.
    pub fn load() -> Self {
        // Check multiple config locations (macOS uses ~/Library/Application Support,
        // but users often expect ~/.config/)
        let candidates: Vec<std::path::PathBuf> = [
            dirs::config_dir().map(|d| d.join("farx").join("config.toml")),
            dirs::home_dir().map(|d| d.join(".config").join("farx").join("config.toml")),
        ]
        .into_iter()
        .flatten()
        .collect();

        let path = match candidates.iter().find(|p| p.exists()) {
            Some(p) => p.clone(),
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
