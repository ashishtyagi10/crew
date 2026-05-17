use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;

type Bindings = HashMap<(KeyCode, KeyModifiers), Action>;

/// Ctrl combos, terminal, swap, history, bookmarks, goto, filter, clipboard.
pub(super) fn build_ctrl_combos(panel: &mut Bindings) {
    panel.insert(
        (KeyCode::PageUp, KeyModifiers::CONTROL),
        Action::ParentDirectory,
    );
    panel.insert(
        (KeyCode::PageDown, KeyModifiers::CONTROL),
        Action::EnterDirectory,
    );
    panel.insert(
        (KeyCode::Char('\\'), KeyModifiers::CONTROL),
        Action::GotoRoot,
    );
    panel.insert(
        (KeyCode::Char('o'), KeyModifiers::CONTROL),
        Action::TogglePanels,
    );
    panel.insert(
        (KeyCode::Char('l'), KeyModifiers::CONTROL),
        Action::ShowInfoPanel,
    );
    panel.insert(
        (KeyCode::Char(' '), KeyModifiers::CONTROL),
        Action::ShowAiBar,
    );

    // Open terminal here
    panel.insert(
        (KeyCode::Char('`'), KeyModifiers::CONTROL),
        Action::OpenTerminalHere,
    );

    // Swap panels
    panel.insert(
        (KeyCode::Char('u'), KeyModifiers::CONTROL),
        Action::SwapPanels,
    );

    // Directory history
    panel.insert((KeyCode::Left, KeyModifiers::ALT), Action::HistoryBack);
    panel.insert((KeyCode::Right, KeyModifiers::ALT), Action::HistoryForward);

    // Recent directories
    panel.insert(
        (KeyCode::Char('h'), KeyModifiers::ALT),
        Action::ShowRecentDirectories,
    );

    // Bookmarks
    panel.insert(
        (KeyCode::Char('b'), KeyModifiers::CONTROL),
        Action::ShowBookmarks,
    );
    panel.insert((KeyCode::Char('b'), KeyModifiers::ALT), Action::AddBookmark);

    // Go to directory
    panel.insert(
        (KeyCode::Char('g'), KeyModifiers::CONTROL),
        Action::GotoDirectoryDialog,
    );

    // Filter
    panel.insert(
        (KeyCode::Char('f'), KeyModifiers::CONTROL),
        Action::ToggleFilter,
    );

    // Clipboard
    panel.insert(
        (KeyCode::Char('y'), KeyModifiers::CONTROL),
        Action::CopyPathToClipboard,
    );
    panel.insert(
        (KeyCode::Char('y'), KeyModifiers::ALT),
        Action::CopyNameToClipboard,
    );
}
