use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

mod ctrl_combos;
mod function_keys;
mod navigation;
mod parse;
mod sort_tabs;
mod tools;

#[cfg(test)]
mod tests;

pub struct KeyMap {
    pub global: HashMap<(KeyCode, KeyModifiers), Action>,
    pub panel: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl KeyMap {
    /// Build the default FAR Manager keybindings.
    pub fn far_defaults() -> Self {
        let mut global = HashMap::new();
        let mut panel = HashMap::new();

        function_keys::build_global(&mut global);
        function_keys::build_function_keys(&mut panel);
        function_keys::build_shift_function_keys(&mut panel);
        function_keys::build_alt_function_keys(&mut panel);

        navigation::build_navigation(&mut panel);
        navigation::build_selection(&mut panel);

        ctrl_combos::build_ctrl_combos(&mut panel);

        tools::build_tools(&mut panel);
        tools::build_mask_selection(&mut panel);

        sort_tabs::build_sort(&mut panel);
        sort_tabs::build_tabs(&mut panel);
        sort_tabs::build_misc(&mut panel);

        KeyMap { global, panel }
    }

    /// Resolve a key event in panel context.
    /// Checks the panel map first, then the global map.
    /// Unbound character keys fall through to the command line.
    pub fn resolve_panel(&self, key: &KeyEvent) -> Action {
        let lookup = (key.code, key.modifiers);
        if let Some(action) = self.panel.get(&lookup) {
            return action.clone();
        }
        if let Some(action) = self.global.get(&lookup) {
            return action.clone();
        }

        // Handle Shift+Arrow/navigation specially — macOS terminals may add
        // extra modifier bits (SUPER, etc.) alongside SHIFT, so we check
        // .contains() instead of exact match.
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Up => return Action::SelectUp,
                KeyCode::Down => return Action::SelectDown,
                KeyCode::PageUp => return Action::SelectPageUp,
                KeyCode::PageDown => return Action::SelectPageDown,
                KeyCode::Home => return Action::SelectHome,
                KeyCode::End => return Action::SelectEnd,
                _ => {}
            }
        }

        // Fall through: route printable characters to the command line
        match (key.code, key.modifiers) {
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                Action::CommandLineInput(ch)
            }
            (KeyCode::Backspace, KeyModifiers::NONE) => Action::CommandLineBackspace,
            (KeyCode::Enter, KeyModifiers::NONE) => {
                // Enter: if command line has input, execute it; otherwise enter directory
                Action::CommandLineEnterOrDir
            }
            _ => Action::Noop,
        }
    }

    /// Resolve a key event in global context.
    /// Checks the global map, then returns Noop.
    pub fn resolve_global(&self, key: &KeyEvent) -> Action {
        let lookup = (key.code, key.modifiers);
        if let Some(action) = self.global.get(&lookup) {
            return action.clone();
        }
        Action::Noop
    }

    /// Apply user-configured keybinding overrides from config.
    /// Key format: "Ctrl+A", "Alt+B", "F5", "Shift+F4", "Enter", "Space"
    /// Action format: action name matching Action enum variants (case-insensitive).
    pub fn apply_overrides(&mut self, overrides: &HashMap<String, String>) {
        for (key_str, action_str) in overrides {
            if let Some((code, mods)) = parse::parse_key_combo(key_str) {
                if let Some(action) = parse::parse_action(action_str) {
                    self.panel.insert((code, mods), action);
                }
            }
        }
    }
}
