//! Hint mode's two ends: the chord that opens it, and what a picked label
//! does. The mode itself — what is labelled, and which letters mean what —
//! lives in [`crate::hints`]; this is only the app's side of it.
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::hints::{Kind, Press};

/// A key as hint mode sees it. winit's `KeyEvent` is `#[non_exhaustive]` and
/// cannot be built in a test, so the routing is split here — the same shape
/// `ChatInput` takes for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HintKey {
    Typed(char),
    Escape,
    /// An arrow, a function key, anything that is not a label letter.
    Other,
}

impl CrewApp {
    /// Cmd+E: label the focused pane. A pane with nothing on it to reach says
    /// so rather than opening a mode that would eat the next key.
    pub(crate) fn open_hints(&mut self) {
        let i = self.focused;
        let Some(pane) = self.panes.get(i) else {
            return;
        };
        let cells = pane.cells(true);
        let rows = crate::gridrows::grid_lines(&cells, pane.grid.cols, pane.grid.rows);
        match crate::hints::open(i, &rows) {
            true => self.input.focused = false,
            false => self.set_status("nothing on this pane to reach"),
        }
    }

    /// Give a key to the live hint mode. Returns whether it was consumed —
    /// while the mode is on it takes every key, exactly like the `/find` bar,
    /// because the letters ARE the interface.
    pub(crate) fn hint_key(&mut self, event: &KeyEvent) -> bool {
        if !crate::hints::active() {
            return false;
        }
        if !event.state.is_pressed() {
            return true;
        }
        let k = match &event.logical_key {
            Key::Named(NamedKey::Escape) => HintKey::Escape,
            Key::Character(s) => match s.chars().next() {
                Some(c) => HintKey::Typed(c),
                None => HintKey::Other,
            },
            _ => HintKey::Other,
        };
        self.hint_input(k)
    }

    /// [`CrewApp::hint_key`] over a key the tests can build. Always `true`
    /// while the mode is on: the letters ARE the interface.
    pub(crate) fn hint_input(&mut self, k: HintKey) -> bool {
        if !crate::hints::active() {
            return false;
        }
        match k {
            HintKey::Typed(c) => {
                if let Some(Press::Pick(target, open)) = crate::hints::press(c) {
                    match open {
                        true => self.open_hint_target(&target),
                        false => self.copy_text(target.text.clone()),
                    }
                }
            }
            // Escape, or a key that is not a label letter: the mode ends
            // rather than swallowing keys the pane wanted.
            HintKey::Escape | HintKey::Other => crate::hints::close(),
        }
        self.redraw();
        true
    }

    /// A capital label opens its target instead of copying it: a URL leaves
    /// for the browser, a file opens here, and a hash has nowhere to go — so
    /// it is copied, which is what you wanted a commit id for anyway.
    fn open_hint_target(&mut self, t: &crate::hints::Target) {
        match t.kind {
            // `safe_link` is the same guard a Cmd+click goes through: only
            // schemes that are safe to hand an OS opener.
            Kind::Url => match crate::openurl::safe_link(&t.text) {
                Some(url) => {
                    let _ = open::that_detached(url);
                    self.set_status(format!("opened {url}"));
                }
                None => self.set_status(format!("not a link crew will open: {}", t.text)),
            },
            Kind::Path => {
                if !self.open_hint_path(&t.text) {
                    self.set_status(format!("can't open {}", t.text));
                }
            }
            Kind::Hash => self.copy_text(t.text.clone()),
        }
    }
}

#[cfg(test)]
#[path = "hintact_tests.rs"]
mod hintact_tests;
