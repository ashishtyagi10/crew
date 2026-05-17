use serde::{Deserialize, Serialize};

mod ai;
mod general;
mod load;
mod paths;
mod ui;

pub use ai::AiConfig;
pub use general::GeneralConfig;
pub use ui::{PanelConfig, UiConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub ui: UiConfig,
    pub panels: PanelConfig,
    pub ai: AiConfig,
    pub keybindings: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_values_are_sane() {
        let cfg = AppConfig::default();
        assert!(cfg.general.confirm_delete);
        assert_eq!(cfg.ui.tick_rate_ms, 250);
        assert_eq!(cfg.panels.default_sort, "name");
        assert!(!cfg.ai.enabled);
    }

    #[test]
    fn load_reads_home_dot_config_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().to_path_buf();
        let cfg_dir = home.join(".config").join("farx");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("config.toml"),
            r#"
[general]
show_hidden_files = true

[ui]
theme = "dracula"
tick_rate_ms = 100
"#,
        )
        .unwrap();

        let old_home = std::env::var_os("HOME");
        let old_xdg = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("HOME", &home);
        std::env::remove_var("XDG_CONFIG_HOME");

        let loaded = AppConfig::load();

        if let Some(v) = old_home {
            std::env::set_var("HOME", v);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(v) = old_xdg {
            std::env::set_var("XDG_CONFIG_HOME", v);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        assert!(loaded.general.show_hidden_files);
        assert_eq!(loaded.ui.theme, "dracula");
        assert_eq!(loaded.ui.tick_rate_ms, 100);
    }
}
