//! The keys the foot shows, where you are: in groups (move, act, view,
//! more), the most used first in each, a wider gap ([`GAP`]) between them
//! (the reader loop, round 1: "navigation should be simple").

use winit::keyboard::NamedKey;

pub(crate) type Key = (&'static str, &'static str);

/// The key a keycap names, as a press: a letter, `esc`, `enter`.
pub(crate) fn key_of(k: &str) -> Option<winit::keyboard::Key> {
    use winit::keyboard::Key as Press;
    match k {
        "esc" => Some(Press::Named(NamedKey::Escape)),
        "enter" => Some(Press::Named(NamedKey::Enter)),
        k if k.chars().count() == 1 => Some(Press::Character(k.into())),
        _ => None,
    }
}
pub(crate) type Keys = &'static [Key];

/// Where one group ends and the next begins.
pub(crate) const GAP: Key = ("", "");

pub(crate) const SETUP: Keys = &[
    ("1\u{2013}5", "open the console"),
    ("enter", "sign in"),
    ("esc", "close"),
];
pub(crate) const SIGNING_IN: Keys = &[("esc", "cancel")];
pub(crate) const LIST: Keys = &[
    ("enter", "read"),
    ("/", "search"),
    ("R", "refresh"),
    GAP,
    ("e", "archive"),
    ("d", "delete"),
    ("r", "reply"),
    ("z", "undo"),
    ("s", "star"),
    ("m", "read/unread"),
    GAP,
    ("a", "ask smith"),
    ("t", "todo"),
    ("L", "lines"),
    ("o", "gmail"),
];
pub(crate) const READER: Keys = &[
    ("esc", "back"),
    ("n", "next"),
    ("p", "previous"),
    GAP,
    ("r", "reply"),
    ("e", "archive"),
    ("d", "delete"),
    ("z", "undo"),
    ("s", "star"),
    ("m", "unread"),
    GAP,
    ("q", "quoted replies"),
    ("i", "pictures"),
    ("v", "text"),
    ("w", "window"),
    GAP,
    ("a", "ask smith"),
    ("t", "todo"),
    ("u", "unsubscribe"),
    ("o", "gmail"),
];
