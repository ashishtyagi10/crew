//! Terminal event listener. We only care about the program-set window title
//! (OSC 0/2); everything else is ignored.
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};

#[derive(Clone)]
pub(crate) struct TitleListener {
    pub title: Arc<Mutex<String>>,
}

impl EventListener for TitleListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(t) => *self.title.lock().unwrap() = t,
            Event::ResetTitle => self.title.lock().unwrap().clear(),
            _ => {}
        }
    }
}
