//! Dead keys and composed letters: Option+e then e types `é` in a shell, vim
//! or Claude Code, as does a German or French layout's own accent key.
//!
//! winit only combines a Mac's dead-key sequences for a window that takes
//! IME input ("IME must be enabled to receive text-input where dead-key
//! sequences are combined", `Window::set_ime_allowed`), and crew never asked:
//! the accent was dropped and the letter came through bare. With IME on, the
//! accent waits as preedit (no key event at all) and the letter it finishes
//! arrives as `Ime::Commit("é")` instead of a key press. Plain letters, Ctrl
//! combos, Cmd chords and the named keys still arrive as key presses, since
//! AppKit leaves them nothing to compose.
//!
//! Option as Meta keeps IME off: there Option+e must reach `altmeta` as a key
//! press to send `ESC e`, and the input system would hold it as an accent.
//! Off a Mac winit folds the accent into the key press's `text`, which
//! `session::typed_bytes` prefers.
//!
//! The preedit accent is not drawn: the pane shows the letter once it lands.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use winit::event::Ime;
use winit::window::Window;

/// Whether the window takes composed text from the input system: on a Mac,
/// unless Option is Meta (see the module note).
pub(crate) fn ime_wanted(mac: bool, option_as_meta: bool) -> bool {
    mac && !option_as_meta
}

/// Put [`ime_wanted`] into effect on `window` — at creation, and again when
/// the Option-as-Meta setting changes.
pub(crate) fn allow_ime(window: &Window, option_as_meta: bool) {
    window.set_ime_allowed(ime_wanted(cfg!(target_os = "macos"), option_as_meta));
}

impl CrewApp {
    /// A committed composition goes where a typed letter would: the input bar
    /// when it holds focus, else the focused terminal pane (broadcast
    /// included). Other panes and the help overlay take keys, not text, and
    /// are left alone. Preedit changes carry nothing to type.
    pub(crate) fn on_ime(&mut self, ime: Ime) {
        let Ime::Commit(text) = ime else {
            return;
        };
        if text.is_empty() || text.chars().any(char::is_control) || self.help_open {
            return;
        }
        self.last_input_ms = crate::anim::now_ms();
        if self.input.focused {
            self.input.text.push_str(&text);
            self.input.menu_sel = 0;
            self.input.hist_pos = None;
        } else if let Some(PaneContent::Terminal(_)) =
            self.panes.get(self.focused).map(|p| &p.content)
        {
            self.write_to_terminals(text.as_bytes());
        }
        self.redraw();
    }
}

#[cfg(test)]
#[path = "composeinput_tests.rs"]
mod tests;
