use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;

type Bindings = HashMap<(KeyCode, KeyModifiers), Action>;

/// Panel navigation: arrows, paging, home/end, tree expand/collapse.
pub(super) fn build_navigation(panel: &mut Bindings) {
    panel.insert((KeyCode::Up, KeyModifiers::NONE), Action::CursorUp);
    panel.insert((KeyCode::Down, KeyModifiers::NONE), Action::CursorDown);
    panel.insert((KeyCode::PageUp, KeyModifiers::NONE), Action::CursorPageUp);
    panel.insert(
        (KeyCode::PageDown, KeyModifiers::NONE),
        Action::CursorPageDown,
    );
    panel.insert((KeyCode::Home, KeyModifiers::NONE), Action::CursorHome);
    panel.insert((KeyCode::End, KeyModifiers::NONE), Action::CursorEnd);
    // Right/Left arrow = tree expand/collapse (in tree view) or enter/parent (in list view)
    panel.insert((KeyCode::Right, KeyModifiers::NONE), Action::TreeExpand);
    panel.insert((KeyCode::Left, KeyModifiers::NONE), Action::TreeCollapse);
    // Enter is handled specially in resolve_panel: if command line has input,
    // it executes; otherwise it enters the directory. So we don't bind it here.
    panel.insert((KeyCode::Insert, KeyModifiers::NONE), Action::ToggleSelect);
}

/// Selection bindings: Space toggle, Shift+Arrow, Ctrl+A/D, Alt+Arrow.
pub(super) fn build_selection(panel: &mut Bindings) {
    // Space = toggle select + move down (works on all terminals including macOS)
    panel.insert(
        (KeyCode::Char(' '), KeyModifiers::NONE),
        Action::ToggleSelect,
    );

    // Alt+Up/Down = select while moving (macOS friendly)
    panel.insert((KeyCode::Up, KeyModifiers::ALT), Action::SelectUp);
    panel.insert((KeyCode::Down, KeyModifiers::ALT), Action::SelectDown);

    // Ctrl+A = select all, Ctrl+D = deselect all
    panel.insert(
        (KeyCode::Char('a'), KeyModifiers::CONTROL),
        Action::SelectAll,
    );
    panel.insert(
        (KeyCode::Char('d'), KeyModifiers::CONTROL),
        Action::DeselectAll,
    );

    // Shift+Arrow for terminals that support it
    panel.insert((KeyCode::Up, KeyModifiers::SHIFT), Action::SelectUp);
    panel.insert((KeyCode::Down, KeyModifiers::SHIFT), Action::SelectDown);
    panel.insert((KeyCode::PageUp, KeyModifiers::SHIFT), Action::SelectPageUp);
    panel.insert(
        (KeyCode::PageDown, KeyModifiers::SHIFT),
        Action::SelectPageDown,
    );
    panel.insert((KeyCode::Home, KeyModifiers::SHIFT), Action::SelectHome);
    panel.insert((KeyCode::End, KeyModifiers::SHIFT), Action::SelectEnd);
}
