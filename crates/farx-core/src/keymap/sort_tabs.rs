use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;

type Bindings = HashMap<(KeyCode, KeyModifiers), Action>;

/// Sort modes (Ctrl+F3..F6).
pub(super) fn build_sort(panel: &mut Bindings) {
    panel.insert((KeyCode::F(3), KeyModifiers::CONTROL), Action::SortByName);
    panel.insert(
        (KeyCode::F(4), KeyModifiers::CONTROL),
        Action::SortByExtension,
    );
    panel.insert((KeyCode::F(5), KeyModifiers::CONTROL), Action::SortBySize);
    panel.insert((KeyCode::F(6), KeyModifiers::CONTROL), Action::SortByDate);
}

/// Tab management bindings.
pub(super) fn build_tabs(panel: &mut Bindings) {
    panel.insert((KeyCode::Char('t'), KeyModifiers::CONTROL), Action::NewTab);
    panel.insert(
        (KeyCode::Char('w'), KeyModifiers::CONTROL),
        Action::CloseTab,
    );
    panel.insert((KeyCode::Tab, KeyModifiers::CONTROL), Action::NextTab);
    // Alt+1..9 for switching tabs
    for i in 1..=9u8 {
        panel.insert(
            (KeyCode::Char((b'0' + i) as char), KeyModifiers::ALT),
            Action::SwitchTab(i as usize - 1),
        );
    }
}

/// Compare directories, toggle hidden, refresh panel.
pub(super) fn build_misc(panel: &mut Bindings) {
    panel.insert(
        (KeyCode::F(9), KeyModifiers::CONTROL),
        Action::CompareDirectories,
    );
    panel.insert(
        (KeyCode::Char('h'), KeyModifiers::CONTROL),
        Action::ToggleHidden,
    );
    panel.insert(
        (KeyCode::Char('r'), KeyModifiers::CONTROL),
        Action::RefreshPanel,
    );
}
