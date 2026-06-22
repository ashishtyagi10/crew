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
}

impl EventListener for TermEvents {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(t) => *self.title.lock().unwrap() = t,
            Event::ResetTitle => self.title.lock().unwrap().clear(),
            Event::ClipboardStore(_, text) => *self.clipboard.lock().unwrap() = Some(text),
            Event::Bell => self.bell.store(true, Ordering::Relaxed),
            _ => {}
        }
    }
}
