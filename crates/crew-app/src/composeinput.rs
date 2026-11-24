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
//! IME is on only while a shell or the input bar holds focus — the two that
//! take the commit. Option as Meta keeps it off: there Option+e must reach
//! `altmeta` as a key press to send `ESC e`, and the input system would hold
//! it as an accent.
//!
//! A view that takes IME also gets the Mac's press-and-hold accent picker:
//! holding `l` in vim would open a menu instead of repeating. Crew turns it
//! off for itself ([`hold_repeats`]), as Alacritty and kitty do.
//! Off a Mac winit folds the accent into the key press's `text`, which
//! `session::typed_bytes` prefers.
//!
//! The preedit — a waiting accent, or kana an input method is still turning
//! into kanji — is drawn at the caret by [`crate::imepreedit`].
use crate::app::CrewApp;
use crate::pane::PaneContent;
use winit::event::Ime;

/// Whether the window takes composed text from the input system: on a Mac,
/// unless Option is Meta (see the module note).
pub(crate) fn ime_wanted(mac: bool, option_as_meta: bool) -> bool {
    mac && !option_as_meta
}

/// Holding a key repeats it rather than offering accents: registers
/// `ApplePressAndHoldEnabled = NO` for this process only — a registration
/// default, never written to the user's preferences, and outranked by an
/// explicit `defaults write` should someone want the picker back.
#[cfg(target_os = "macos")]
pub(crate) fn hold_repeats() {
    use objc2::runtime::AnyObject;
    use objc2_foundation::{ns_string, NSDictionary, NSNumber, NSUserDefaults};
    let no = NSNumber::new_bool(false);
    let value: &AnyObject = &no;
    let defaults = NSDictionary::from_slices(&[ns_string!("ApplePressAndHoldEnabled")], &[value]);
    // SAFETY: the dictionary maps a string key to an NSNumber, the shape
    // `registerDefaults:` documents.
    unsafe { NSUserDefaults::standardUserDefaults().registerDefaults(&defaults) };
}

impl CrewApp {
    /// Whether what holds focus takes composed text: the input bar or a
    /// terminal pane, with no mode up that reads keys as commands (help,
    /// hints, the Explorer tree). Chat, editors, settings and inbox read key
    /// presses only, and a dead key held as preedit would never reach them.
    pub(crate) fn takes_text(&self) -> bool {
        if self.help_open || crate::hints::active() || self.nav_has_keys() {
            return false;
        }
        let focused = self.panes.get(self.focused).map(|p| &p.content);
        self.input.focused || matches!(focused, Some(PaneContent::Terminal(_)))
    }

    /// Keep the window's IME on exactly while [`ime_wanted`] and
    /// [`Self::takes_text`] both hold. Run after every window event: winit
    /// asks the flag before crew sees a key, so it must already be right for
    /// the next one. Told only on a change. A composition whose surface lost
    /// focus is dropped here too.
    pub(crate) fn sync_ime(&mut self) {
        let want =
            ime_wanted(cfg!(target_os = "macos"), self.config.option_as_meta) && self.takes_text();
        // A composition stays with the surface it began on (`imepreedit`).
        if self.ime.follow(self.ime_owner()) {
            self.redraw();
        }
        if self.ime.on == Some(want) {
            return;
        }
        if let Some(w) = &self.window {
            w.set_ime_allowed(want);
            self.ime.on = Some(want);
            // Turned back on, the window has forgotten where the caret is.
            self.ime.area = None;
        }
    }

    /// A committed composition goes where a typed letter would: the input bar
    /// when it holds focus, else the focused terminal pane (broadcast
    /// included). A preedit carries nothing to type: it is kept and drawn
    /// until it commits or is dropped (`imepreedit`).
    pub(crate) fn on_ime(&mut self, ime: Ime) {
        // With nothing taking text there is nothing to compose for.
        let held = match self.ime_owner() {
            Some(owner) => self.ime.note(&ime, owner),
            None => self.ime.preedit.take().is_some(),
        };
        if held {
            self.redraw();
        }
        let Ime::Commit(text) = ime else {
            return;
        };
        if text.is_empty() || text.chars().any(char::is_control) || !self.takes_text() {
            return;
        }
        self.last_input_ms = crate::anim::now_ms();
        if self.input.focused {
            self.input.text.push_str(&text);
            self.input.menu_sel = 0;
            self.input.hist_pos = None;
        } else {
            self.write_to_terminals(text.as_bytes());
        }
        self.redraw();
    }
}

#[cfg(test)]
#[path = "composeinput_tests.rs"]
mod tests;
