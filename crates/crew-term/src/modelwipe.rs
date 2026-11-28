//! Running the parser in pieces cut at the sequences that drop pictures
//! (see `crate::wipescan`). Child module of `model`, split for the 200-line cap.
use alacritty_terminal::term::TermMode;

use super::TermCore;
use crate::wipescan::{Hit, ImageWipe, WipeScan};

/// The watcher, the wipes the app has not collected yet, and the kitty
/// keyboard stack depths the same watcher counts (`modelkitty`).
#[derive(Default)]
pub(super) struct Wipes {
    scan: WipeScan,
    pending: Vec<ImageWipe>,
    pub(crate) kitty: modelkitty::KittyStacks,
    /// How many titles crew holds on the pushed stack (`modeltitle`).
    title_depth: u8,
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
            match hit {
                Hit::KittyPush if !self.kitty_push() => {
                    // Its final byte swapped for CAN, which aborts it.
                    self.parse(b"\x18");
                    at = end + 1;
                    continue;
                }
                Hit::KittyPush => continue,
                Hit::KittyPop(n) => {
                    self.kitty_pop(n);
                    continue;
                }
                Hit::TitlePush(true) => {
                    self.title_push();
                    continue;
                }
                Hit::TitlePop(true) if self.title_pop() => continue,
                // Icon-only, or a pop with nothing of crew's to pop: aborted.
                Hit::TitlePush(_) | Hit::TitlePop(_) => {
                    self.parse(b"\x18");
                    at = end + 1;
                    continue;
                }
                Hit::Reset => {
                    self.kitty_reset();
                    self.title_reset();
                }
                _ => {}
            }
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
                _ => {}
            }
        }
        self.parse(&bytes[at..]);
        self.answer_colors();
    }

    /// Drop the matching pictures not yet collected, and keep the wipe for
    /// the ones the app already holds.
    pub(super) fn wipe(&mut self, w: ImageWipe) {
        self.images.retain(|p| w.keeps(p.line, p.alt, p.cmd.id));
        self.wipes.pending.push(w);
    }

    /// Drop the pictures whose anchor line has left the scrollback. One
    /// pending wipe of this kind is enough: a later count covers an earlier.
    pub(super) fn wipe_evicted(&mut self) {
        let w = ImageWipe::Evicted(self.evicted);
        self.images.retain(|p| w.keeps(p.line, p.alt, p.cmd.id));
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

#[path = "modelkitty.rs"]
mod modelkitty;
pub(super) use modelkitty::POP_ALL;
#[path = "modeltitle.rs"]
mod modeltitle;

#[cfg(test)]
#[path = "modelwipe_tests.rs"]
mod modelwipe_tests;
