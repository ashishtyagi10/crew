//! Terminal event listener. We capture the program-set window title (OSC 0/2)
//! and clipboard-store requests (OSC 52); everything else is ignored.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};

/// Shared state captured from terminal events (cloned into the alacritty `Term`).
#[derive(Clone, Default)]
pub(crate) struct TermEvents {
    pub title: Arc<Mutex<String>>,
    pub clipboard: Arc<Mutex<Option<String>>>,
    pub bell: Arc<AtomicBool>,
    /// Bytes owed back to the child on the pty: answers to color queries
    /// (OSC 10/11) and status reports (DSR) raised while parsing its output.
    pub replies: Arc<Mutex<String>>,
}

impl TermEvents {
    /// Drain the pending query replies (`None` when empty).
    pub(crate) fn take_replies(&self) -> Option<String> {
        let mut r = self.replies.lock().unwrap();
        (!r.is_empty()).then(|| std::mem::take(&mut *r))
    }
}

impl EventListener for TermEvents {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(t) => *self.title.lock().unwrap() = t,
            Event::ResetTitle => self.title.lock().unwrap().clear(),
            Event::ClipboardStore(_, text) => *self.clipboard.lock().unwrap() = Some(text),
            Event::Bell => self.bell.store(true, Ordering::Relaxed),
            // Status reports (DSR cursor position, device attributes, …) the
            // child expects as terminal input.
            Event::PtyWrite(text) => self.replies.lock().unwrap().push_str(&text),
            // Color queries (OSC 4/10/11/12): answer from the active theme so
            // agent CLIs probing the background pick the matching light/dark
            // palette instead of assuming a dark terminal.
            Event::ColorRequest(index, format) => {
                if let Some(rgb) = crate::color::query_color(index) {
                    self.replies.lock().unwrap().push_str(&format(rgb));
                }
            }
            _ => {}
        }
    }
}
