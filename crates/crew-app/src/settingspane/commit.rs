//! Draft-commit logic for the settings form: parse and clamp each edit buffer
//! into the draft config, refresh the buffers on focus moves, and build the
//! final config on Save.
use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL, FIELDS};
use crate::config::CrewConfig;

/// Commit the selected family from the filtered list into the draft.
pub(crate) fn commit_family(p: &mut SettingsPane) {
    let list = p.filtered();
    if let Some(name) = list.get(p.family_sel) {
        if name == DEFAULT_FAMILY_LABEL {
            p.draft.font_family = None;
            p.family_query = DEFAULT_FAMILY_LABEL.to_string();
        } else {
            p.draft.font_family = Some(name.clone());
            p.family_query = name.clone();
        }
    }
    p.family_open = false;
}

/// Parse and clamp the currently-focused editable field into the draft.
pub(crate) fn commit_field(p: &mut SettingsPane) {
    match p.focused_field() {
        Field::FontFamily => commit_family(p),
        Field::FontSize => {
            let v = p.size_buf.parse::<f32>().unwrap_or(p.draft.font_size);
            p.draft.font_size = v.clamp(12.0, 32.0);
        }
        Field::NavWidth => {
            let v = p.nav_buf.parse::<f32>().unwrap_or(p.draft.nav_width);
            p.draft.nav_width = crate::navresize::width_at(v);
        }
        Field::Accent => commit_accent(p),
        Field::NotifyMinSecs => {
            let v = p
                .minsecs_buf
                .parse::<u64>()
                .unwrap_or(p.draft.notify_min_secs);
            p.draft.notify_min_secs = v.clamp(1, 3600);
        }
        Field::Budget5h => {
            p.draft.usage_budget_5h =
                super::tokens::commit(&p.budget5h_buf, p.draft.usage_budget_5h);
        }
        Field::Budget7d => {
            p.draft.usage_budget_7d =
                super::tokens::commit(&p.budget7d_buf, p.draft.usage_budget_7d);
        }
        Field::NotifyPatterns => {
            p.draft.notify_patterns = p
                .patterns_buf
                .split('\n')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        _ => {}
    }
    refresh_bufs(p);
}

fn commit_accent(p: &mut SettingsPane) {
    let raw = p.accent_buf.trim();
    if raw.is_empty() {
        // Cleared → fall back to the built-in accent.
        p.draft.accent = None;
    } else if let Some((r, g, b)) = crate::palette::parse_hex(raw) {
        // Store the canonical `#rrggbb` form.
        p.draft.accent = Some(format!("#{r:02x}{g:02x}{b:02x}"));
    }
    // Invalid hex → keep the previous value (refresh restores the buffer).
}

/// Mirror the draft back into every display buffer (after a commit or focus
/// move, so half-typed edits never linger).
pub(crate) fn refresh_bufs(p: &mut SettingsPane) {
    p.size_buf = format!("{}", p.draft.font_size as i32);
    p.nav_buf = format!("{}", p.draft.nav_width as i32);
    p.accent_buf = p.draft.accent.clone().unwrap_or_default();
    p.minsecs_buf = format!("{}", p.draft.notify_min_secs);
    p.patterns_buf = p.draft.notify_patterns.join("\n");
    p.budget5h_buf = super::tokens::label(p.draft.usage_budget_5h);
    p.budget7d_buf = super::tokens::label(p.draft.usage_budget_7d);
    p.family_query = p
        .draft
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_FAMILY_LABEL.to_string());
}

/// Move focus to the next/previous field, closing the dropdown and refreshing
/// the display buffers from the draft.
pub(crate) fn move_focus(p: &mut SettingsPane, back: bool) {
    // Step through the order the form is DRAWN in (`form::tab_order`), not
    // the order the fields were declared in. `p.focus` still indexes `FIELDS`,
    // which is the set; the walk is over the layout, which is the sequence.
    let order = super::form::tab_order(p.cols.get());
    let here = order.iter().position(|f| *f == p.focused_field());
    let n = order.len();
    let next = match here {
        Some(i) if back => (i + n - 1) % n,
        Some(i) => (i + 1) % n,
        // Focused on something the layout does not place: start at the top
        // rather than refusing to move.
        None => 0,
    };
    p.focus = FIELDS
        .iter()
        .position(|f| *f == order[next])
        .unwrap_or(p.focus);
    p.family_open = false;
    p.family_sel = 0;
    refresh_bufs(p);
}

pub(crate) fn build_config(p: &SettingsPane) -> CrewConfig {
    p.draft.clone().clamped()
}

/// Escape closes the font dropdown if it's open, otherwise cancels the form.
pub(crate) fn escape(p: &mut SettingsPane) -> Option<SettingsAction> {
    if p.family_open {
        p.family_open = false;
        None
    } else {
        Some(SettingsAction::Cancel)
    }
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod tests;
