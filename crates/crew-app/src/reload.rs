//! `/reload`: re-read the config file from disk and apply it live, picking up
//! edits made outside the in-app settings pane (font family/size, sidebar width
//! and visibility). It does not write the config back, so external formatting
//! and comments are preserved.
use crate::app::CrewApp;
use crate::config::CrewConfig;

impl CrewApp {
    /// Reload `config.toml` from disk and apply it (without persisting).
    pub(crate) fn reload_config(&mut self) {
        self.apply_config(CrewConfig::load());
        self.set_status("config reloaded");
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;
    use crate::config::CrewConfig;

    #[test]
    fn apply_config_adopts_values_without_a_renderer() {
        let mut app = CrewApp::default();
        let cfg = CrewConfig {
            font_size: 19.0,
            show_nav: false,
            ..CrewConfig::default()
        };
        // No renderer in tests: the font calls are skipped, but config is adopted
        // and a relayout/redraw is safe to request.
        app.apply_config(cfg);
        assert_eq!(app.config.font_size, 19.0);
        assert!(!app.config.show_nav);
    }
}
