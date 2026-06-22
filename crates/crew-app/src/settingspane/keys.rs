//! Key reduction for the settings form: Tab navigation, text editing, the
//! font-family combobox, and Save/Cancel activation.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::{Field, SettingsAction, SettingsPane, DEFAULT_FAMILY_LABEL, FIELDS};
use crate::config::CrewConfig;

/// Handle one key; return an action when Save/Cancel is triggered.
pub(crate) fn reduce(p: &mut SettingsPane, key: &KeyEvent, shift: bool) -> Option<SettingsAction> {
    if !key.state.is_pressed() {
        return None;
    }
    match &key.logical_key {
        Key::Named(NamedKey::Escape) => return escape(p),
        Key::Named(NamedKey::Tab) => {
            commit_field(p);
            move_focus(p, shift);
            return None;
        }
        _ => {}
    }
    match p.focused_field() {
        Field::FontFamily => family_key(p, key),
        Field::FontSize => number_key(p, key, true),
        Field::NavWidth => number_key(p, key, false),
        Field::ShowNav => toggle_key(p, key),
        Field::Save => button_key(p, key, true),
        Field::Cancel => button_key(p, key, false),
    }
}

fn family_key(p: &mut SettingsPane, key: &KeyEvent) -> Option<SettingsAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Enter) => {
            commit_family(p);
        }
        Key::Named(NamedKey::ArrowDown) => {
            p.family_open = true;
            let n = p.filtered().len().max(1);
            p.family_sel = (p.family_sel + 1).min(n - 1);
        }
        Key::Named(NamedKey::ArrowUp) => {
            p.family_open = true;
            p.family_sel = p.family_sel.saturating_sub(1);
        }
        Key::Named(NamedKey::Backspace) => {
            p.family_query.pop();
            p.family_open = true;
            p.family_sel = 0;
        }
        Key::Named(NamedKey::Space) => push_family(p, ' '),
        Key::Character(s) => {
            if let Some(c) = s.chars().next() {
                push_family(p, c);
            }
        }
        _ => {}
    }
    None
}

fn number_key(p: &mut SettingsPane, key: &KeyEvent, is_size: bool) -> Option<SettingsAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Enter) => {
            commit_field(p);
            move_focus(p, false);
        }
        Key::Named(NamedKey::Backspace) => {
            buf_mut(p, is_size).pop();
        }
        Key::Character(s) => {
            if let Some(c) = s.chars().next() {
                if c.is_ascii_digit() {
                    buf_mut(p, is_size).push(c);
                }
            }
        }
        _ => {}
    }
    None
}

fn toggle_key(p: &mut SettingsPane, key: &KeyEvent) -> Option<SettingsAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Enter) => move_focus(p, false),
        Key::Named(NamedKey::Space)
        | Key::Named(NamedKey::ArrowLeft)
        | Key::Named(NamedKey::ArrowRight) => p.draft.show_nav = !p.draft.show_nav,
        _ => {}
    }
    None
}

fn button_key(p: &mut SettingsPane, key: &KeyEvent, is_save: bool) -> Option<SettingsAction> {
    match &key.logical_key {
        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) if is_save => {
            commit_field(p);
            Some(SettingsAction::Apply(build_config(p)))
        }
        Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) => Some(SettingsAction::Cancel),
        _ => None,
    }
}

fn push_family(p: &mut SettingsPane, c: char) {
    p.family_query.push(c);
    p.family_open = true;
    p.family_sel = 0;
}

fn buf_mut(p: &mut SettingsPane, is_size: bool) -> &mut String {
    if is_size {
        &mut p.size_buf
    } else {
        &mut p.nav_buf
    }
}

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
        Field::FontSize => {
            let v = p.size_buf.parse::<f32>().unwrap_or(p.draft.font_size);
            p.draft.font_size = v.clamp(12.0, 32.0);
            p.size_buf = format!("{}", p.draft.font_size as i32);
        }
        Field::NavWidth => {
            let v = p.nav_buf.parse::<f32>().unwrap_or(p.draft.nav_width);
            p.draft.nav_width = v.clamp(160.0, 320.0);
            p.nav_buf = format!("{}", p.draft.nav_width as i32);
        }
        Field::FontFamily => commit_family(p),
        _ => {}
    }
}

/// Move focus to the next/previous field, closing the dropdown and refreshing
/// the display buffers from the draft.
pub(crate) fn move_focus(p: &mut SettingsPane, back: bool) {
    let n = FIELDS.len();
    p.focus = if back {
        (p.focus + n - 1) % n
    } else {
        (p.focus + 1) % n
    };
    p.family_open = false;
    p.family_sel = 0;
    p.size_buf = format!("{}", p.draft.font_size as i32);
    p.nav_buf = format!("{}", p.draft.nav_width as i32);
    p.family_query = p
        .draft
        .font_family
        .clone()
        .unwrap_or_else(|| DEFAULT_FAMILY_LABEL.to_string());
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
