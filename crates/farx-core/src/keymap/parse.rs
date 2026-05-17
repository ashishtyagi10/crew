use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;

/// Parse a key combination string like "Ctrl+A", "Alt+F7", "F5", "Space".
pub(super) fn parse_key_combo(s: &str) -> Option<(KeyCode, KeyModifiers)> {
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
pub(super) fn parse_action(s: &str) -> Option<Action> {
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
        "focusleftpanel" | "focusleft" => Some(Action::FocusLeftPanel),
        "focusrightpanel" | "focusright" => Some(Action::FocusRightPanel),
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
