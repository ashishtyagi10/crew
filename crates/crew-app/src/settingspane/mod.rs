//! Settings form pane: a two-column bento of fieldset cards (Appearance /
//! Window / Notifications) covering **every user-configurable property**,
//! with boxed inputs, checkboxes, a notify-patterns text area, Tab/wheel
//! navigation, a type-to-search font-family dropdown, and Save (Cmd+S /
//! Alt+S) / Cancel (Esc).
mod cards;
mod commit;
mod cycle;
mod dropdown;
mod fields;
mod fit;
mod form;
mod keys;
mod labels;
mod pairing;
mod render;
mod tokens;

use crew_render::CellView;
#[cfg(test)]
pub(crate) use cycle::cycle_value;
use winit::event::KeyEvent;

use crate::config::CrewConfig;
pub(crate) use fields::{Field, FIELDS};

/// Label shown for "no explicit family — use the system monospace".
pub(crate) const DEFAULT_FAMILY_LABEL: &str = "System monospace";

/// Outcome of a key press in the settings form.
pub enum SettingsAction {
    /// Save: apply this config and close the pane. Boxed: `CrewConfig` has
    /// grown past clippy's large-enum-variant threshold (it now carries
    /// `model_recents`), and `Cancel` is a bare unit variant — without the
    /// indirection every `Option<SettingsAction>` pays for the config's full
    /// size even on the no-op path.
    Apply(Box<CrewConfig>),
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
    /// Editable accent hex (e.g. `#00ffa0`); empty means "use the built-in".
    pub(crate) accent_buf: String,
    /// Paper-grain amplitude (`0.0`–`2.0`, one decimal).
    pub(crate) grain_buf: String,
    /// Window opacity as a whole percentage (`35`–`100`), so the number the
    /// user types is the number they see rather than a 0-1 fraction.
    pub(crate) opacity_buf: String,
    /// `auto`'s light-hours window as typed, `HH:MM`. Normalised to
    /// zero-padded form on every refresh, so `7:5` reads back as `07:05`.
    pub(crate) light_from_buf: String,
    pub(crate) light_to_buf: String,
    /// Minimum command runtime (seconds) before a "finished" notification.
    pub(crate) minsecs_buf: String,
    /// Watched output substrings, one per line (text area).
    pub(crate) patterns_buf: String,
    /// The two usage budgets in MILLIONS of tokens (see `tokens`), so the
    /// number typed is the number said rather than eight digits.
    pub(crate) budget5h_buf: String,
    pub(crate) budget7d_buf: String,
}

impl SettingsPane {
    pub fn new(cfg: CrewConfig, families: Vec<String>) -> Self {
        let family_query = cfg
            .font_family
            .clone()
            .unwrap_or_else(|| DEFAULT_FAMILY_LABEL.to_string());
        let size_buf = format!("{}", cfg.font_size as i32);
        let nav_buf = format!("{}", cfg.nav_width as i32);
        let accent_buf = cfg.accent.clone().unwrap_or_default();
        let grain_buf = format!("{:.1}", cfg.paper_grain);
        let opacity_buf = format!("{}", (cfg.window_opacity * 100.0).round() as i32);
        let (light_from_buf, light_to_buf) = commit::light_bufs(&cfg);
        let minsecs_buf = format!("{}", cfg.notify_min_secs);
        let patterns_buf = cfg.notify_patterns.join("\n");
        let budget5h_buf = tokens::label(cfg.usage_budget_5h);
        let budget7d_buf = tokens::label(cfg.usage_budget_7d);
        Self {
            draft: cfg,
            families,
            focus: 0,
            family_query,
            family_open: false,
            family_sel: 0,
            size_buf,
            nav_buf,
            accent_buf,
            grain_buf,
            opacity_buf,
            light_from_buf,
            light_to_buf,
            minsecs_buf,
            patterns_buf,
            budget5h_buf,
            budget7d_buf,
        }
    }

    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        render::render(self, cols, rows)
    }

    pub fn on_key(&mut self, key: &KeyEvent, shift: bool) -> Option<SettingsAction> {
        keys::reduce(self, key, shift)
    }

    /// Cmd+S / Alt+S: commit the focused field and save the whole form.
    pub fn save(&mut self) -> SettingsAction {
        commit::commit_field(self);
        SettingsAction::Apply(Box::new(commit::build_config(self)))
    }

    /// Mouse-wheel / page scroll: move the open font dropdown's selection, or
    /// otherwise step field focus (committing each field on the way). Positive
    /// `lines` moves toward the top.
    pub fn scroll(&mut self, lines: i32) {
        if self.family_open {
            let n = self.filtered().len() as i64;
            if n > 0 {
                self.family_sel = (self.family_sel as i64 - lines as i64).clamp(0, n - 1) as usize;
            }
            return;
        }
        let up = lines > 0;
        for _ in 0..lines.unsigned_abs().min(FIELDS.len() as u32) {
            if (up && self.focus == 0) || (!up && self.focus == FIELDS.len() - 1) {
                break;
            }
            commit::commit_field(self);
            commit::move_focus(self, up);
        }
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
#[path = "mod_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "form_tests.rs"]
mod form_tests;
