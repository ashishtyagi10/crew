//! When a picture stops belonging to the screen: `clear`, a reset, and the
//! alternate screen coming and going.
//!
//! A picture is anchored to an absolute buffer line (`PlacedImage::line`),
//! which only means "the text it arrived in" while that text exists. `clear`
//! (`ESC[2J ESC[3J`) throws the text away and empties the history, so in a
//! young pane the old anchor lands back on the screen and the picture is drawn
//! over the new prompt. A file manager's preview is placed on the alternate
//! screen, whose lines number from 0 again, and it would be drawn over the
//! shell after the program quits. kitty drops pictures at these same points,
//! so programs already expect it.
//!
//! The parser gives no word of either, so the bytes are watched here, and the
//! position of each hit is kept: the parser is run up to it, so the wipe sees
//! the screen as it was before the erase (`TermCore::advance`).

/// Pictures to drop, in the order the program asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageWipe {
    /// Erase the screen (`ESC[2J`): the pictures on it, on that screen only.
    /// `line` is the first absolute line of the screen at the time.
    Screen { line: u64, alt: bool },
    /// Erase the scrollback (`ESC[3J`), a kitty delete of everything
    /// (`a=d`, `d=a`), or a full reset.
    All,
    /// A kitty delete by id (`a=d,d=i,i=N`): the pictures sent as `i=N`,
    /// and only those — a previewer clearing its last preview must not take
    /// every other picture in the scrollback with it. Id 0 names none: a
    /// picture sent without one can't be deleted by id.
    Id(u32),
    /// The alternate screen was entered (`true`) or left: every picture
    /// placed on it goes, and the primary's show again only on the primary.
    Alt(bool),
    /// Every line before this absolute one has been thrown off the top of a
    /// full scrollback: a picture anchored there has no text left to sit on.
    Evicted(u64),
}

impl ImageWipe {
    /// Whether a picture at `line`, placed on the alternate screen or not,
    /// sent with kitty id `id` (0 for none), survives this wipe. The one rule
    /// both halves (core and app) apply.
    pub fn keeps(self, line: u64, alt: bool, id: u32) -> bool {
        match self {
            Self::Screen { line: top, alt: on } => alt != on || line < top,
            Self::All => false,
            Self::Id(n) => n == 0 || id != n,
            Self::Alt(_) => !alt,
            Self::Evicted(top) => alt || line >= top,
        }
    }
}

/// What a watched sequence asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    EraseScreen,
    EraseSaved,
    AltScreen(bool),
    Reset,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Esc,
    Csi,
}

/// A tiny CSI reader that only looks for ED 2/3, DECSET 47/1047/1049 and RIS,
/// carrying its state across reads.
#[derive(Default)]
pub(crate) struct WipeScan {
    state: State,
    private: bool,
    /// Set by an intermediate byte (`CSI 2 SP J` is not an erase).
    odd: bool,
    params: Vec<u32>,
}

/// Parameters kept per sequence; a longer list is noise, not a request.
const MAX_PARAMS: usize = 16;

impl WipeScan {
    /// Every hit in `bytes`, each with the index of the byte that ends it.
    pub(crate) fn feed(&mut self, bytes: &[u8]) -> Vec<(usize, Hit)> {
        let mut hits = Vec::new();
        for (i, &b) in bytes.iter().enumerate() {
            match (self.state, b) {
                (_, 0x1b) => self.state = State::Esc,
                (State::Ground, _) => {}
                (State::Esc, b'[') => {
                    self.state = State::Csi;
                    (self.private, self.odd) = (false, false);
                    self.params.clear();
                    self.params.push(0);
                }
                (State::Esc, b'c') => {
                    self.state = State::Ground;
                    hits.push((i, Hit::Reset));
                }
                (State::Esc, _) => self.state = State::Ground,
                (State::Csi, b'?') if self.params == [0] && !self.private => self.private = true,
                (State::Csi, b'0'..=b'9') => {
                    let last = self.params.last_mut().expect("never empty");
                    *last = last.saturating_mul(10).saturating_add(u32::from(b - b'0'));
                }
                (State::Csi, b';') if self.params.len() < MAX_PARAMS => self.params.push(0),
                (State::Csi, 0x20..=0x3f) => self.odd = true,
                (State::Csi, 0x40..=0x7e) => {
                    self.state = State::Ground;
                    if let Some(hit) = self.hit(b) {
                        hits.push((i, hit));
                    }
                }
                // C0 controls inside a CSI run without ending it.
                (State::Csi, _) => {}
            }
        }
        hits
    }

    fn hit(&self, fin: u8) -> Option<Hit> {
        if self.odd {
            return None;
        }
        match (self.private, fin) {
            (false, b'J') => match self.params[..] {
                [2] => Some(Hit::EraseScreen),
                [3] => Some(Hit::EraseSaved),
                _ => None,
            },
            (true, b'h' | b'l') => self
                .params
                .iter()
                .any(|p| matches!(p, 47 | 1047 | 1049))
                .then_some(Hit::AltScreen(fin == b'h')),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "wipescan_tests.rs"]
mod wipescan_tests;
