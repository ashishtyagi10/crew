//! The keys a kitty-flagged program saw go down, and where each one went.
//!
//! A release is forwarded only for a press the program saw (a crew chord ate
//! the press, so its release must not leak), and when focus leaves the pane
//! (window blur, Cmd+Tab, a pane switch) the releases still owed go to THAT
//! pane: the OS delivers them to whoever has focus by then, so otherwise a
//! Shift held across the switch stays down in nvim and its release lands on
//! the wrong program. Kitty does the same on focus loss.
use std::collections::HashMap;

use winit::keyboard::{Key, KeyCode};

use crate::keyenc::Mods;
use crate::kittyreport::{encode, Ev, Kind};

/// What it takes to encode a key's release later, as the press saw it.
#[derive(Clone)]
pub(crate) struct Held {
    /// The pane the press went to, by `Pane::born_ms` (panes reorder, so an
    /// index would name another pane after a close).
    pub pane: u64,
    pub flags: u8,
    pub logical: Key,
    pub bare: Key,
    pub numpad: bool,
    pub mods: Mods,
    pub mac: bool,
    pub meta: bool,
}

thread_local! {
    /// The physical keys pressed while a kitty-flagged program held focus.
    static DOWN: std::cell::RefCell<HashMap<KeyCode, Held>> = std::cell::RefCell::default();
}

/// Whether an event goes on to the encoder: every press, and a release only
/// of a key whose press this program saw. With no flags nothing is kept.
pub(crate) fn admit(
    flags: u8,
    pressed: bool,
    code: Option<KeyCode>,
    held: impl FnOnce() -> Held,
) -> bool {
    DOWN.with(|d| {
        let mut d = d.borrow_mut();
        match (flags, pressed) {
            (0, _) => d.clear(),
            (_, true) => {
                if let Some(c) = code {
                    d.insert(c, held());
                }
            }
            (_, false) => return code.is_some_and(|c| d.remove(&c).is_some()),
        }
        true
    })
}

/// Take every entry `owned` claims and `keep` does not, and return the
/// release each owes: `(pane, bytes)`. Entries whose program never asked for
/// event types owe nothing but are still forgotten.
pub(crate) fn take_stale(
    owned: impl Fn(u64) -> bool,
    keep: impl Fn(u64) -> bool,
) -> Vec<(u64, Vec<u8>)> {
    DOWN.with(|d| {
        let mut d = d.borrow_mut();
        let gone: Vec<KeyCode> = d
            .iter()
            .filter(|(_, h)| owned(h.pane) && !keep(h.pane))
            .map(|(c, _)| *c)
            .collect();
        let mut out = Vec::new();
        for c in gone {
            let Some(h) = d.remove(&c) else { continue };
            let e = Ev {
                logical: &h.logical,
                bare: &h.bare,
                text: None,
                code: Some(c),
                numpad: false,
                kind: Kind::Release,
            };
            let e = Ev {
                numpad: h.numpad,
                ..e
            };
            if let Some(b) = encode(h.flags, &e, h.mods, h.mac, h.meta) {
                out.push((h.pane, b));
            }
        }
        out
    })
}

#[cfg(test)]
#[path = "kittyheld_tests.rs"]
mod tests;
