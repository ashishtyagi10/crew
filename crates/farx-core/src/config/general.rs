use serde::{Deserialize, Serialize};

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
