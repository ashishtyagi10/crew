use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub theme: String,
    pub tick_rate_ms: u64,
    pub date_format: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "tokyo-night".to_string(),
            tick_rate_ms: 250,
            date_format: "%Y-%m-%d %H:%M".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    pub directories_first: bool,
    pub default_sort: String,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            directories_first: true,
            default_sort: "name".to_string(),
        }
    }
}
