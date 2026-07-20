//! Key reduction for the settings form: Tab navigation, text editing, the
//! font-family combobox, cycling toggles/pickers, and Save/Cancel activation.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use super::commit::{build_config, commit_family, commit_field, escape, move_focus};
use super::{Field, SettingsAction, SettingsPane};

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
        Field::Save => button_key(p, key, true),
        Field::Cancel => button_key(p, key, false),
        f if buf_of(p, f).is_some() => edit_key(p, key),
        _ => cycle_key(p, key),
    }
}

/// The edit buffer behind a text/number field, if it has one.
pub(crate) fn buf_of(p: &mut SettingsPane, f: Field) -> Option<&mut String> {
    match f {
        Field::FontSize => Some(&mut p.size_buf),
        Field::NavWidth => Some(&mut p.nav_buf),
        Field::Accent => Some(&mut p.accent_buf),
        Field::PaperGrain => Some(&mut p.grain_buf),
        Field::NotifyMinSecs => Some(&mut p.minsecs_buf),
        Field::NotifyPatterns => Some(&mut p.patterns_buf),
        _ => None,
    }
}

/// Whether `c` may be typed into the field's buffer (currently `buf`).
fn allowed(f: Field, buf: &str, c: char) -> bool {
    match f {
        Field::FontSize | Field::NavWidth | Field::NotifyMinSecs => c.is_ascii_digit(),
        Field::PaperGrain => c.is_ascii_digit() || (c == '.' && !buf.contains('.')),
        Field::Accent => (c == '#' || c.is_ascii_hexdigit()) && buf.len() < 7,
        Field::NotifyPatterns => !c.is_control(),
        _ => false,
    }
}

/// Shared editor for every buffered field: Enter commits and advances,
/// Backspace deletes, and permitted characters append.
fn edit_key(p: &mut SettingsPane, key: &KeyEvent) -> Option<SettingsAction> {
    let f = p.focused_field();
    match &key.logical_key {
        Key::Named(NamedKey::Enter) if f == Field::NotifyPatterns => {
            // The patterns field is a text area: Enter starts a new pattern.
            if let Some(buf) = buf_of(p, f) {
                buf.push('\n');
            }
        }
        Key::Named(NamedKey::Enter) => {
            commit_field(p);
            move_focus(p, false);
        }
        Key::Named(NamedKey::Backspace) => {
            if let Some(buf) = buf_of(p, f) {
                buf.pop();
            }
        }
        Key::Named(NamedKey::Space) => push_char(p, f, ' '),
        Key::Character(s) => {
            if let Some(c) = s.chars().next() {
                push_char(p, f, c);
            }
        }
        _ => {}
    }
    None
}

fn push_char(p: &mut SettingsPane, f: Field, c: char) {
    let ok = buf_of(p, f).is_some_and(|b| allowed(f, b, c));
    if ok {
        if let Some(buf) = buf_of(p, f) {
            buf.push(c);
        }
    }
}

/// Toggles and pickers: Space / ← / → cycle the value (Left steps backward on
/// the theme), Enter advances to the next field.
fn cycle_key(p: &mut SettingsPane, key: &KeyEvent) -> Option<SettingsAction> {
    let back = matches!(&key.logical_key, Key::Named(NamedKey::ArrowLeft));
    match &key.logical_key {
        Key::Named(NamedKey::Enter) => move_focus(p, false),
        Key::Named(NamedKey::Space)
        | Key::Named(NamedKey::ArrowLeft)
        | Key::Named(NamedKey::ArrowRight) => cycle_value(p, back),
        _ => {}
    }
    None
}

/// Flip the focused toggle, or step the theme picker through the three
/// consolidated themes (`dark`, `light`, `crt`).
fn cycle_value(p: &mut SettingsPane, back: bool) {
    let field = p.focused_field();
    let d = &mut p.draft;
    match field {
        Field::ShowNav => d.show_nav = !d.show_nav,
        Field::PaperTexture => d.paper_texture = !d.paper_texture,
        Field::Maximized => d.maximized = !d.maximized,
        Field::Notify => d.notify = !d.notify,
        Field::NotifyAgentDone => d.notify_agent_done = !d.notify_agent_done,
        Field::NotifyBell => d.notify_bell = !d.notify_bell,
        Field::NotifyExit => d.notify_exit = !d.notify_exit,
        Field::Theme => {
            let modes = crew_theme::THEME_MODES;
            let cur = d
                .theme
                .as_deref()
                .and_then(crew_theme::parse_selection)
                .and_then(|sel| match sel {
                    crew_theme::Selection::Mode(m) => modes.iter().position(|&x| x == m),
                    crew_theme::Selection::Fixed(_) => None,
                })
                .unwrap_or(0);
            let next = if back {
                (cur + modes.len() - 1) % modes.len()
            } else {
                (cur + 1) % modes.len()
            };
            d.theme = Some(modes[next].as_str().to_string());
        }
        _ => {}
    }
}

pub(crate) fn family_key(p: &mut SettingsPane, key: &KeyEvent) -> Option<SettingsAction> {
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
