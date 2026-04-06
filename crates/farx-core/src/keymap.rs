use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::types::PanelSide;

pub struct KeyMap {
    pub global: HashMap<(KeyCode, KeyModifiers), Action>,
    pub panel: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl KeyMap {
    /// Build the default FAR Manager keybindings.
    pub fn far_defaults() -> Self {
        let mut global = HashMap::new();
        let mut panel = HashMap::new();

        // ── Global bindings ────────────────────────────────────────────
        global.insert((KeyCode::F(1), KeyModifiers::NONE), Action::ShowHelp);
        global.insert((KeyCode::F(9), KeyModifiers::NONE), Action::ShowMenu);
        global.insert((KeyCode::F(10), KeyModifiers::NONE), Action::Quit);
        global.insert(
            (KeyCode::F(11), KeyModifiers::NONE),
            Action::ShowPluginCommands,
        );
        global.insert((KeyCode::F(12), KeyModifiers::NONE), Action::ShowScreenList);

        // ── Panel: function keys ───────────────────────────────────────
        panel.insert((KeyCode::F(3), KeyModifiers::NONE), Action::ViewFile);
        panel.insert((KeyCode::F(4), KeyModifiers::NONE), Action::EditFile);
        panel.insert((KeyCode::F(5), KeyModifiers::NONE), Action::CopyDialog);
        panel.insert((KeyCode::F(6), KeyModifiers::NONE), Action::MoveDialog);
        panel.insert((KeyCode::F(7), KeyModifiers::NONE), Action::MkDirDialog);
        panel.insert((KeyCode::F(8), KeyModifiers::NONE), Action::DeleteDialog);

        // ── Panel: Shift+F keys ────────────────────────────────────────
        panel.insert(
            (KeyCode::F(4), KeyModifiers::SHIFT),
            Action::CreateFileDialog,
        );
        panel.insert((KeyCode::F(5), KeyModifiers::SHIFT), Action::CopySameDir);
        panel.insert((KeyCode::F(6), KeyModifiers::SHIFT), Action::RenameDialog);

        // ── Panel: Alt+F keys ──────────────────────────────────────────
        panel.insert(
            (KeyCode::F(1), KeyModifiers::ALT),
            Action::ShowDriveMenu(PanelSide::Left),
        );
        panel.insert(
            (KeyCode::F(2), KeyModifiers::ALT),
            Action::ShowDriveMenu(PanelSide::Right),
        );
        panel.insert((KeyCode::F(7), KeyModifiers::ALT), Action::ShowSearchDialog);

        // ── Panel: navigation ──────────────────────────────────────────
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
        panel.insert((KeyCode::Tab, KeyModifiers::NONE), Action::SwitchPanel);
        panel.insert((KeyCode::Insert, KeyModifiers::NONE), Action::ToggleSelect);

        // ── Panel: Selection ──────────────────────────────────────────
        // Space = toggle select + move down (works on all terminals including macOS)
        // This is the primary selection method.
        // Insert is kept for keyboards that have it.
        // Shift+Arrow kept for terminals that support it (iTerm2, Kitty, etc.)
        panel.insert(
            (KeyCode::Char(' '), KeyModifiers::NONE),
            Action::ToggleSelect,
        );

        // Ctrl+Up/Down = select while moving (macOS friendly)
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

        // ── Panel: Ctrl combos ─────────────────────────────────────────
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

        // ── Directory history ────────────────────────────────────────────
        panel.insert((KeyCode::Left, KeyModifiers::ALT), Action::HistoryBack);
        panel.insert((KeyCode::Right, KeyModifiers::ALT), Action::HistoryForward);

        // ── Bookmarks ────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('b'), KeyModifiers::CONTROL),
            Action::ShowBookmarks,
        );
        panel.insert((KeyCode::Char('b'), KeyModifiers::ALT), Action::AddBookmark);

        // ── Filter ──────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('f'), KeyModifiers::CONTROL),
            Action::ToggleFilter,
        );

        // ── Clipboard ────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('y'), KeyModifiers::CONTROL),
            Action::CopyPathToClipboard,
        );

        // ── Directory size ──────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('s'), KeyModifiers::ALT),
            Action::CalculateDirSize,
        );

        // ── Undo ─────────────────────────────────────────────────────────
        panel.insert((KeyCode::Char('z'), KeyModifiers::CONTROL), Action::Undo);

        // ── Batch rename ────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('m'), KeyModifiers::CONTROL),
            Action::BatchRename,
        );

        // ── Fuzzy finder ──────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('p'), KeyModifiers::CONTROL),
            Action::ShowFuzzyFinder,
        );

        // ── Quick actions ─────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Enter, KeyModifiers::ALT),
            Action::ShowQuickActions,
        );

        // ── Duplicate finder ──────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('d'), KeyModifiers::ALT),
            Action::FindDuplicates,
        );

        // ── Disk usage treemap ──────────────────────────────────────────
        panel.insert((KeyCode::Char('t'), KeyModifiers::ALT), Action::ShowTreemap);

        // ── Checksums ─────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('k'), KeyModifiers::ALT),
            Action::ShowChecksums,
        );

        // ── Archives ─────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('e'), KeyModifiers::ALT),
            Action::ExtractArchive,
        );
        panel.insert(
            (KeyCode::Char('c'), KeyModifiers::ALT),
            Action::CompressSelection,
        );

        // ── Symlink ─────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('l'), KeyModifiers::ALT),
            Action::CreateSymlinkDialog,
        );

        // ── Select/Deselect by mask ────────────────────────────────────
        // Alt+= (easier to press) and Alt++ (Shift+= on most keyboards)
        panel.insert(
            (KeyCode::Char('+'), KeyModifiers::ALT),
            Action::SelectByMaskDialog,
        );
        panel.insert(
            (
                KeyCode::Char('+'),
                KeyModifiers::ALT.union(KeyModifiers::SHIFT),
            ),
            Action::SelectByMaskDialog,
        );
        panel.insert(
            (KeyCode::Char('='), KeyModifiers::ALT),
            Action::SelectByMaskDialog,
        );
        panel.insert(
            (KeyCode::Char('-'), KeyModifiers::ALT),
            Action::DeselectByMaskDialog,
        );

        // ── Sort modes (Ctrl+F3..F6) ────────────────────────────────────
        panel.insert((KeyCode::F(3), KeyModifiers::CONTROL), Action::SortByName);
        panel.insert(
            (KeyCode::F(4), KeyModifiers::CONTROL),
            Action::SortByExtension,
        );
        panel.insert((KeyCode::F(5), KeyModifiers::CONTROL), Action::SortBySize);
        panel.insert((KeyCode::F(6), KeyModifiers::CONTROL), Action::SortByDate);

        // ── Toggle hidden, refresh ───────────────────────────────────────
        panel.insert(
            (KeyCode::Char('h'), KeyModifiers::CONTROL),
            Action::ToggleHidden,
        );
        panel.insert(
            (KeyCode::Char('r'), KeyModifiers::CONTROL),
            Action::RefreshPanel,
        );

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
}
