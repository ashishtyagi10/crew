//! Terminal event listener. We capture the program-set window title (OSC 0/2)
//! and clipboard-store requests (OSC 52); everything else is ignored.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::vte::ansi::Rgb;

/// A colour query waiting for its answer: where in [`TermEvents::replies`]
/// it was asked, the palette slot, and alacritty's reply formatter.
pub(crate) type ColorAsk = (usize, usize, Arc<dyn Fn(Rgb) -> String + Sync + Send>);

/// alacritty's DECRPM for a mode it doesn't know, asked about 2031.
const UNKNOWN_2031: &str = "\x1b[?2031;0$y";

/// Shared state captured from terminal events (cloned into the alacritty `Term`).
#[derive(Clone, Default)]
pub(crate) struct TermEvents {
    pub title: Arc<Mutex<String>>,
    pub clipboard: Arc<Mutex<Option<String>>>,
    pub bell: Arc<AtomicBool>,
    /// Bytes owed back to the child on the pty: answers to color queries
    /// (OSC 10/11) and status reports (DSR) raised while parsing its output.
    pub replies: Arc<Mutex<String>>,
    /// The grid and cell size, kept by `TermCore` for `CSI 14 t` (`modelsize`);
    /// set from its first moment (`TermCore::new`).
    pub size: Arc<Mutex<Option<WindowSize>>>,
    /// Colour queries (OSC 4/10/11/12) not answered yet. The listener can't
    /// see the palette a program set, so `TermCore` answers these after the
    /// parser runs, from `term.colors()` (see `modelcolor`).
    pub color_asks: Arc<Mutex<Vec<ColorAsk>>>,
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
            // Not alacritty's "2031 unknown": crew's scheme scanner already
            // answered that DECRQM ("supported"), and a program reading the
            // last answer would turn its notifications off.
            Event::PtyWrite(text) if text == UNKNOWN_2031 => {}
            Event::PtyWrite(text) => self.replies.lock().unwrap().push_str(&text),
            // Color queries (OSC 4/10/11/12): held, with their place among
            // the other replies, for `TermCore` to answer from the palette.
            Event::ColorRequest(index, format) => {
                let at = self.replies.lock().unwrap().len();
                self.color_asks.lock().unwrap().push((at, index, format));
            }
            Event::TextAreaSizeRequest(format) => {
                if let Some(size) = *self.size.lock().unwrap() {
                    self.replies.lock().unwrap().push_str(&format(size));
                }
            }
            _ => {}
        }
    }
}
