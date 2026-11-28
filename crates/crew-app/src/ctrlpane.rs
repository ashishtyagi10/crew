//! The Ctrl keys a pane keeps for itself off a Mac (docs/plan/window-keys.md,
//! "Ctrl, by where the focus is"). Alt is crew's key there, so plain Ctrl
//! belongs to whatever is typed into: a terminal gets every Ctrl byte, with
//! Windows Terminal's two exceptions (Ctrl+C copies a selection, Ctrl+V
//! pastes). The editors and text fields get their save and find. On a Mac
//! none of this exists: Cmd does it, and Ctrl+C is ^C, as ever.
use winit::keyboard::{Key, KeyCode, ModifiersState};

use crate::app::CrewApp;
use crate::pane::PaneContent;

/// What a plain Ctrl+key asks of the pane under focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CtrlKey {
    /// Ctrl+C: copy the selection, when there is one.
    Copy,
    /// Ctrl+V: paste the clipboard, as Cmd+V / Alt+V do.
    Paste,
    /// Ctrl+S: save the settings form or the inbox draft.
    Save,
    /// Ctrl+F: find, from an editing document.
    Find,
}

/// The [`CtrlKey`] a key press is, or `None`. Pure over the key so both
/// platforms are tested on one: a Mac never has one, and Shift, Alt (AltGr is
/// Ctrl+Alt) or Super with Ctrl make it something else.
pub(crate) fn ctrl_key(key: &Key, mods: ModifiersState, mac: bool) -> Option<CtrlKey> {
    if mac || mods != ModifiersState::CONTROL {
        return None;
    }
    let Key::Character(s) = key else {
        return None;
    };
    match s.to_ascii_lowercase().as_str() {
        "c" => Some(CtrlKey::Copy),
        "v" => Some(CtrlKey::Paste),
        "s" => Some(CtrlKey::Save),
        "f" => Some(CtrlKey::Find),
        _ => None,
    }
}

/// The key as the letter printed on it, for Ctrl's four: on a Russian or Greek
/// layout Ctrl+C arrives as `с`, and copy, paste, save and find were lost there
/// (`cmd_chord` reads the physical key the same way).
pub(crate) fn latin(key: &Key, code: Option<KeyCode>) -> Key {
    let ascii = matches!(key, Key::Character(s) if s.is_ascii());
    let letter = match code {
        Some(KeyCode::KeyC) => "c",
        Some(KeyCode::KeyV) => "v",
        Some(KeyCode::KeyS) => "s",
        Some(KeyCode::KeyF) => "f",
        _ => return key.clone(),
    };
    match (ascii, key) {
        (false, Key::Character(_)) => Key::Character(letter.into()),
        _ => key.clone(),
    }
}

/// The key `event` names, through [`latin`].
pub(crate) fn event_key(event: &winit::event::KeyEvent) -> Key {
    let code = match event.physical_key {
        winit::keyboard::PhysicalKey::Code(c) => Some(c),
        winit::keyboard::PhysicalKey::Unidentified(_) => None,
    };
    latin(&event.logical_key, code)
}

/// [`ctrl_key`] as the chord string `cmd_chord` gives the mail window, which
/// has no pane to ask: only the paste is one there.
pub(crate) fn paste_chord(key: &Key, mods: ModifiersState, mac: bool) -> Option<String> {
    (ctrl_key(key, mods, mac) == Some(CtrlKey::Paste)).then(|| "v".to_string())
}

impl CrewApp {
    /// Run the Ctrl key a press is, off a Mac. `true` when it was taken, so
    /// it must not also reach the pane (a terminal's ^V, and ^C while text is
    /// selected; its ^S and ^F are never taken).
    pub(crate) fn ctrl_pane_key(&mut self, key: &Key, mods: ModifiersState) -> bool {
        match ctrl_key(key, mods, cfg!(target_os = "macos")) {
            Some(k) => self.run_ctrl_key(k),
            None => false,
        }
    }

    pub(crate) fn run_ctrl_key(&mut self, k: CtrlKey) -> bool {
        let focused = self.focused;
        if k == CtrlKey::Paste {
            self.paste();
            return true;
        }
        let in_pane = !self.input.focused;
        match (k, self.panes.get_mut(focused).map(|p| &mut p.content)) {
            // A chat's Ctrl+C clears or interrupts; a stale selection must
            // not take that away.
            (CtrlKey::Copy, Some(PaneContent::Chat(_))) => false,
            // With the input bar focused, Ctrl+C is the bar's: a selection
            // left in the pane behind it must not take the key.
            (CtrlKey::Copy, _) if in_pane => {
                let Some(text) = self.pane_selection_text(focused) else {
                    return false;
                };
                self.copy_text(text);
                self.cell_sel = None;
                if let Some(PaneContent::Terminal(t)) =
                    self.panes.get_mut(focused).map(|p| &mut p.content)
                {
                    t.pty.sel_clear();
                }
                true
            }
            (CtrlKey::Save, Some(PaneContent::Settings(_))) if in_pane => {
                self.save_focused_settings()
            }
            (CtrlKey::Save, Some(PaneContent::Inbox(_))) if in_pane => self.inbox_save_draft(),
            (CtrlKey::Find, Some(PaneContent::View(v))) if in_pane && v.caret.is_some() => {
                self.handle_super_chord("f");
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "ctrlpane_tests.rs"]
mod tests;
