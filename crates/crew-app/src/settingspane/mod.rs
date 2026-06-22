//! Settings form pane: a two-column layout with Tab navigation, a type-to-search
//! font-family dropdown, editable numeric fields, and Save/Cancel buttons.
mod keys;
mod render;

use crew_render::CellView;
use winit::event::KeyEvent;

use crate::config::CrewConfig;

/// Label shown for "no explicit family — use the system monospace".
pub(crate) const DEFAULT_FAMILY_LABEL: &str = "System monospace";

/// Focusable elements of the form, in Tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Field {
    FontFamily,
    FontSize,
    NavWidth,
    ShowNav,
    Save,
    Cancel,
}

pub(crate) const FIELDS: [Field; 6] = [
    Field::FontFamily,
    Field::FontSize,
    Field::NavWidth,
    Field::ShowNav,
    Field::Save,
    Field::Cancel,
];

/// Outcome of a key press in the settings form.
pub enum SettingsAction {
    /// Save: apply this config and close the pane.
    Apply(CrewConfig),
    /// Cancel: discard edits and close the pane.
    Cancel,
}

pub struct SettingsPane {
    pub(crate) draft: CrewConfig,
    pub(crate) families: Vec<String>,
    pub(crate) focus: usize,
    pub(crate) family_query: String,
    pub(crate) family_open: bool,
    pub(crate) family_sel: usize,
    pub(crate) size_buf: String,
    pub(crate) nav_buf: String,
}

impl SettingsPane {
    pub fn new(cfg: CrewConfig, families: Vec<String>) -> Self {
        let family_query = cfg
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FAMILY_LABEL.to_string());
        let size_buf = format!("{}", cfg.font_size as i32);
        let nav_buf = format!("{}", cfg.nav_width as i32);
        Self {
            draft: cfg,
            families,
            focus: 0,
            family_query,
            family_open: false,
            family_sel: 0,
            size_buf,
            nav_buf,
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent, shift: bool) -> Option<SettingsAction> {
        keys::reduce(self, key, shift)
    }

    pub(crate) fn focused_field(&self) -> Field {
        FIELDS[self.focus.min(FIELDS.len() - 1)]
    }

    /// Family names matching the current query (case-insensitive substring),
    /// always led by the system-default label.
    pub(crate) fn filtered(&self) -> Vec<String> {
        let q = self.family_query.to_lowercase();
        let mut out = Vec::new();
        if q.is_empty() || DEFAULT_FAMILY_LABEL.to_lowercase().contains(&q) {
            out.push(DEFAULT_FAMILY_LABEL.to_string());
        }
        for f in &self.families {
            if q.is_empty() || f.to_lowercase().contains(&q) {
                out.push(f.clone());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::keys::{build_config, commit_family, commit_field, escape, move_focus};
    use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL};
    use crate::config::CrewConfig;

    fn pane() -> SettingsPane {
        SettingsPane::new(
            CrewConfig::default(),
            vec!["Menlo".into(), "JetBrains Mono".into()],
        )
    }

    #[test]
    fn filtered_leads_with_default_label() {
        assert_eq!(pane().filtered().first().unwrap(), DEFAULT_FAMILY_LABEL);
    }

    #[test]
    fn filtered_narrows_on_query() {
        let mut p = pane();
        p.family_query = "jet".into();
        assert_eq!(p.filtered(), vec!["JetBrains Mono".to_string()]);
    }

    #[test]
    fn commit_font_size_clamps_low() {
        let mut p = pane();
        p.focus = 1; // FontSize
        p.size_buf = "3".into();
        commit_field(&mut p);
        assert_eq!(p.draft.font_size, 12.0);
        assert_eq!(p.size_buf, "12");
    }

    #[test]
    fn commit_family_sets_draft() {
        let mut p = pane();
        p.family_query = "jet".into();
        p.family_sel = 0;
        commit_family(&mut p);
        assert_eq!(p.draft.font_family.as_deref(), Some("JetBrains Mono"));
        assert!(!p.family_open);
    }

    #[test]
    fn move_focus_wraps_backwards_to_cancel() {
        let mut p = pane();
        move_focus(&mut p, true);
        assert_eq!(p.focused_field(), Field::Cancel);
    }

    #[test]
    fn esc_closes_dropdown_then_cancels() {
        let mut p = pane();
        p.family_open = true;
        assert!(escape(&mut p).is_none()); // first Esc closes the dropdown
        assert!(!p.family_open);
        assert!(matches!(escape(&mut p), Some(SettingsAction::Cancel)));
    }

    #[test]
    fn build_config_returns_edited_draft() {
        let mut p = pane();
        p.focus = 1; // FontSize
        p.size_buf = "20".into();
        commit_field(&mut p);
        assert_eq!(build_config(&p).font_size, 20.0);
    }
}
