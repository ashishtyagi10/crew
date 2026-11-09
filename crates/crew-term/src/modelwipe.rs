//! Running the parser in pieces cut at the sequences that drop pictures
//! (see `crate::wipescan`). Child module of `model`, split for the 200-line cap.
use alacritty_terminal::term::TermMode;

use super::TermCore;
use crate::wipescan::{Hit, ImageWipe, WipeScan};

/// The watcher and the wipes the app has not collected yet.
#[derive(Default)]
pub(super) struct Wipes {
    scan: WipeScan,
    pending: Vec<ImageWipe>,
}

impl TermCore {
    /// Feed `bytes` to the parser, stopping just before the last byte of each
    /// erase or screen switch: up to there the grid is still the one the
    /// program is about to throw away, which is what says which pictures go.
    pub(super) fn advance(&mut self, bytes: &[u8]) {
        let mut at = 0;
        for (end, hit) in self.wipes.scan.feed(bytes) {
            self.parse(&bytes[at..end]);
            at = end;
            let alt = self.term.mode().contains(TermMode::ALT_SCREEN);
            match hit {
                Hit::EraseScreen => self.wipe(ImageWipe::Screen {
                    line: self.screen_line(),
                    alt,
                }),
                Hit::EraseSaved => self.wipe(ImageWipe::All),
                Hit::AltScreen(on) => self.wipe(ImageWipe::Alt(on)),
                Hit::Reset => {
                    self.wipe(ImageWipe::All);
                    self.wipe(ImageWipe::Alt(false));
                }
            }
        }
        self.parse(&bytes[at..]);
    }

    /// Drop the matching pictures not yet collected, and keep the wipe for
    /// the ones the app already holds.
    pub(super) fn wipe(&mut self, w: ImageWipe) {
        self.images.retain(|p| w.keeps(p.line, p.alt));
        self.wipes.pending.push(w);
    }

    /// Drop the pictures whose anchor line has left the scrollback. One
    /// pending wipe of this kind is enough: a later count covers an earlier.
    pub(super) fn wipe_evicted(&mut self) {
        let w = ImageWipe::Evicted(self.evicted);
        self.images.retain(|p| w.keeps(p.line, p.alt));
        match self.wipes.pending.last_mut() {
            Some(last @ ImageWipe::Evicted(_)) => *last = w,
            _ => self.wipes.pending.push(w),
        }
    }

    /// Take the wipes since the last call. Apply them BEFORE the pictures
    /// taken at the same time: those already had them applied.
    pub(crate) fn take_image_wipes(&mut self) -> Vec<ImageWipe> {
        std::mem::take(&mut self.wipes.pending)
    }
}

#[cfg(test)]
#[path = "modelwipe_tests.rs"]
mod modelwipe_tests;
