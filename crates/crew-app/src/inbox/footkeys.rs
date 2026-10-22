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
        "tab" => Some(Press::Named(NamedKey::Tab)),
        "\u{2191}" => Some(Press::Named(NamedKey::ArrowUp)),
        "\u{2193}" => Some(Press::Named(NamedKey::ArrowDown)),
        "del" => Some(Press::Named(NamedKey::Backspace)),
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
/// While a search is typed, every key is the search's.
pub(crate) const SEARCH: Keys = &[("enter", "search"), ("esc", "cancel"), ("del", "erase")];
/// The keycap that sends a reply: the chord the platform sends with.
pub(crate) const SEND: &str = if cfg!(target_os = "macos") {
    "\u{2318}\u{21a9}"
} else {
    "ctrl+\u{21a9}"
};
/// Writing a reply's words; [`FORWARD`] a forward's note, [`ADDRESS`] on
/// its `to` or `cc` line, [`FILES`] on its row of files.
pub(crate) const COMPOSE: Keys = &[
    (SEND, "send"),
    ("esc", "close"),
    GAP,
    ("\u{2191}", "to and cc"),
    ("\u{2193}", "attach"),
    ("tab", "agent smith drafts it"),
];
pub(crate) const FORWARD: Keys = &[
    (SEND, "send"),
    ("esc", "close"),
    GAP,
    ("\u{2191}", "to and cc"),
    ("\u{2193}", "attach"),
];
pub(crate) const FILES: Keys = &[
    (SEND, "send"),
    ("esc", "close"),
    GAP,
    ("\u{2190}\u{2192}", "choose"),
    ("enter", "open or attach"),
    ("del", "take off"),
    ("\u{2191}", "back to the words"),
];
pub(crate) const ADDRESS: Keys = &[(SEND, "send"), ("esc", "close"), GAP, ("tab", "next line")];
pub(crate) const LIST: Keys = &[
    ("enter", "read"),
    ("/", "search"),
    ("R", "refresh"),
    GAP,
    ("e", "archive"),
    ("d", "delete"),
    ("r", "reply"),
    ("f", "forward"),
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
    ("f", "forward"),
    ("e", "archive"),
    ("d", "delete"),
    ("z", "undo"),
    ("s", "star"),
    ("m", "read/unread"),
    GAP,
    ("q", "quoted replies"),
    ("i", "pictures"),
    ("v", "text"),
    ("w", "window"),
    ("x", "all mails"),
    GAP,
    ("a", "ask smith"),
    ("t", "todo"),
    ("u", "unsubscribe"),
    ("o", "gmail"),
];
