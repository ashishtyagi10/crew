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
        panel.insert((KeyCode::F(2), KeyModifiers::NONE), Action::OpenSystemApp);
        panel.insert((KeyCode::F(3), KeyModifiers::NONE), Action::EditFile);
        panel.insert((KeyCode::F(4), KeyModifiers::NONE), Action::SwitchPanel);
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

        // ── Open terminal here ────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('`'), KeyModifiers::CONTROL),
            Action::OpenTerminalHere,
        );

        // ── Swap panels ──────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('u'), KeyModifiers::CONTROL),
            Action::SwapPanels,
        );

        // ── Directory history ────────────────────────────────────────────
        panel.insert((KeyCode::Left, KeyModifiers::ALT), Action::HistoryBack);
        panel.insert((KeyCode::Right, KeyModifiers::ALT), Action::HistoryForward);

        // ── Recent directories ────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('h'), KeyModifiers::ALT),
            Action::ShowRecentDirectories,
        );

        // ── Bookmarks ────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('b'), KeyModifiers::CONTROL),
            Action::ShowBookmarks,
        );
        panel.insert((KeyCode::Char('b'), KeyModifiers::ALT), Action::AddBookmark);

        // ── Go to directory ────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('g'), KeyModifiers::CONTROL),
            Action::GotoDirectoryDialog,
        );

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
        panel.insert(
            (KeyCode::Char('y'), KeyModifiers::ALT),
            Action::CopyNameToClipboard,
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

        // ── AI tools panel ──────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('e'), KeyModifiers::CONTROL),
            Action::ShowAiPanel,
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

        // ── Touch file ────────────────────────────────────────────────────
        panel.insert((KeyCode::Char('w'), KeyModifiers::ALT), Action::TouchFile);

        // ── File statistics ──────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('i'), KeyModifiers::ALT),
            Action::ShowFileStats,
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

        // ── File permissions (chmod) ──────────────────────────────────────
        panel.insert((KeyCode::Char('a'), KeyModifiers::ALT), Action::ChmodDialog);

        // ── Symlink ─────────────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('l'), KeyModifiers::ALT),
            Action::CreateSymlinkDialog,
        );

        // ── Invert selection ────────────────────────────────────────────
        panel.insert(
            (KeyCode::Char('*'), KeyModifiers::ALT),
            Action::InvertSelection,
        );
        panel.insert(
            (
                KeyCode::Char('*'),
                KeyModifiers::ALT.union(KeyModifiers::SHIFT),
            ),
            Action::InvertSelection,
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

        // ── Tabs ────────────────────────────────────────────────────────
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

        // ── Compare directories ──────────────────────────────────────────
        panel.insert(
            (KeyCode::F(9), KeyModifiers::CONTROL),
            Action::CompareDirectories,
        );

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

    /// Apply user-configured keybinding overrides from config.
    /// Key format: "Ctrl+A", "Alt+B", "F5", "Shift+F4", "Enter", "Space"
    /// Action format: action name matching Action enum variants (case-insensitive).
    pub fn apply_overrides(&mut self, overrides: &std::collections::HashMap<String, String>) {
        for (key_str, action_str) in overrides {
            if let Some((code, mods)) = parse_key_combo(key_str) {
                if let Some(action) = parse_action(action_str) {
                    self.panel.insert((code, mods), action);
                }
            }
        }
    }
}

/// Parse a key combination string like "Ctrl+A", "Alt+F7", "F5", "Space".
fn parse_key_combo(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    let mut modifiers = KeyModifiers::NONE;
    let mut key_part = "";

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "option" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            _ => key_part = part,
        }
    }

    let code = match key_part.to_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "space" => KeyCode::Char(' '),
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        s if s.starts_with('f') && s.len() <= 3 => {
            if let Ok(n) = s[1..].parse::<u8>() {
                if (1..=12).contains(&n) {
                    KeyCode::F(n)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some((code, modifiers))
}

/// Parse an action name string to an Action enum.
fn parse_action(s: &str) -> Option<Action> {
    match s.to_lowercase().replace(['-', '_'], "").as_str() {
        "cursorup" => Some(Action::CursorUp),
        "cursordown" => Some(Action::CursorDown),
        "cursorpageup" => Some(Action::CursorPageUp),
        "cursorpagedown" => Some(Action::CursorPageDown),
        "cursorhome" => Some(Action::CursorHome),
        "cursorend" => Some(Action::CursorEnd),
        "enterdirectory" | "enter" => Some(Action::EnterDirectory),
        "parentdirectory" | "parent" => Some(Action::ParentDirectory),
        "gotoroot" => Some(Action::GotoRoot),
        "switchpanel" => Some(Action::SwitchPanel),
        "swappanels" => Some(Action::SwapPanels),
        "opensystemapp" | "open" => Some(Action::OpenSystemApp),
        "copydialog" | "copy" => Some(Action::CopyDialog),
        "movedialog" | "move" => Some(Action::MoveDialog),
        "deletedialog" | "delete" => Some(Action::DeleteDialog),
        "mkdirdialog" | "mkdir" => Some(Action::MkDirDialog),
        "renamedialog" | "rename" => Some(Action::RenameDialog),
        "createfiledialog" | "createfile" | "newfile" => Some(Action::CreateFileDialog),
        "copysamedir" => Some(Action::CopySameDir),
        "toggleselect" | "select" => Some(Action::ToggleSelect),
        "selectall" => Some(Action::SelectAll),
        "deselectall" => Some(Action::DeselectAll),
        "invertselection" | "invert" => Some(Action::InvertSelection),
        "selectbymaskdialog" | "selectbymask" => Some(Action::SelectByMaskDialog),
        "deselectbymaskdialog" | "deselectbymask" => Some(Action::DeselectByMaskDialog),
        "sortbyname" => Some(Action::SortByName),
        "sortbyextension" | "sortbyext" => Some(Action::SortByExtension),
        "sortbysize" => Some(Action::SortBySize),
        "sortbydate" => Some(Action::SortByDate),
        "togglehidden" => Some(Action::ToggleHidden),
        "refreshpanel" | "refresh" => Some(Action::RefreshPanel),
        "viewfile" | "view" => Some(Action::ViewFile),
        "editfile" | "edit" => Some(Action::EditFile),
        "togglepanels" => Some(Action::TogglePanels),
        "showinfopanel" | "info" => Some(Action::ShowInfoPanel),
        "showmenu" | "menu" => Some(Action::ShowMenu),
        "showhelp" | "help" => Some(Action::ShowHelp),
        "showsearchdialog" | "search" | "find" => Some(Action::ShowSearchDialog),
        "showaibar" | "ai" => Some(Action::ShowAiBar),
        "showleditfile" | "gotodirectorydialog" | "goto" => Some(Action::GotoDirectoryDialog),
        "historyback" | "back" => Some(Action::HistoryBack),
        "historyforward" | "forward" => Some(Action::HistoryForward),
        "showrecentdirectories" | "recent" => Some(Action::ShowRecentDirectories),
        "showbookmarks" | "bookmarks" => Some(Action::ShowBookmarks),
        "addbookmark" | "bookmark" => Some(Action::AddBookmark),
        "copypathto" | "copypath" | "yank" => Some(Action::CopyPathToClipboard),
        "copynametoclipboard" | "copyname" => Some(Action::CopyNameToClipboard),
        "openterminalhere" | "terminal" | "term" => Some(Action::OpenTerminalHere),
        "touchfile" | "touch" => Some(Action::TouchFile),
        "togglefilter" | "filter" => Some(Action::ToggleFilter),
        "undo" => Some(Action::Undo),
        "batchrename" => Some(Action::BatchRename),
        "showfuzzyfinder" | "fuzzyfinder" | "ff" => Some(Action::ShowFuzzyFinder),
        "extractarchive" | "extract" => Some(Action::ExtractArchive),
        "compressselection" | "compress" | "zip" => Some(Action::CompressSelection),
        "createsymlinkdialog" | "symlink" | "ln" => Some(Action::CreateSymlinkDialog),
        "showquickactions" | "actions" => Some(Action::ShowQuickActions),
        "findduplicates" | "duplicates" => Some(Action::FindDuplicates),
        "comparedirectories" | "compare" => Some(Action::CompareDirectories),
        "showfilestats" | "stats" => Some(Action::ShowFileStats),
        "showchecksums" | "checksum" => Some(Action::ShowChecksums),
        "showtreemap" | "treemap" => Some(Action::ShowTreemap),
        "calculatedirsize" | "dirsize" | "size" => Some(Action::CalculateDirSize),
        "chmoddialog" | "chmod" | "permissions" => Some(Action::ChmodDialog),
        "difffiles" | "diff" => Some(Action::DiffFiles),
        "newtab" => Some(Action::NewTab),
        "closetab" => Some(Action::CloseTab),
        "nexttab" => Some(Action::NextTab),
        "prevtab" => Some(Action::PrevTab),
        "quit" | "exit" => Some(Action::Quit),
        _ => None,
    }
}
