use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::keymap::KeyMap;

#[test]
fn resolve_panel_falls_back_to_command_line_input() {
    let keymap = KeyMap::far_defaults();

    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(keymap.resolve_panel(&key), Action::CommandLineInput('x'));

    let shifted_key = KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT);
    assert_eq!(
        keymap.resolve_panel(&shifted_key),
        Action::CommandLineInput('X')
    );
}

#[test]
fn resolve_panel_falls_back_to_enter_or_dir() {
    let keymap = KeyMap::far_defaults();
    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(keymap.resolve_panel(&key), Action::CommandLineEnterOrDir);
}

#[test]
fn resolve_panel_unbound_ctrl_combo_is_noop() {
    let keymap = KeyMap::far_defaults();
    let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(keymap.resolve_panel(&key), Action::Noop);
}

#[test]
fn apply_overrides_parses_key_combo_and_action_aliases() {
    let mut keymap = KeyMap::far_defaults();
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("Ctrl+E".to_string(), "show-treemap".to_string());
    keymap.apply_overrides(&overrides);

    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(keymap.resolve_panel(&key), Action::ShowTreemap);
}

#[test]
fn apply_overrides_ignores_invalid_entries() {
    let mut keymap = KeyMap::far_defaults();
    let original = keymap.resolve_panel(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::ALT));

    let mut overrides = std::collections::HashMap::new();
    overrides.insert("NotAKey".to_string(), "show_treemap".to_string());
    overrides.insert("Ctrl+Q".to_string(), "definitely-not-an-action".to_string());
    keymap.apply_overrides(&overrides);

    assert_eq!(
        keymap.resolve_panel(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::ALT)),
        original
    );
    assert_eq!(
        keymap.resolve_panel(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        Action::Noop
    );
}
