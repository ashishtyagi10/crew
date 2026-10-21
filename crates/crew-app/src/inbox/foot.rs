//! The pane's last line: what the keys do where you are, or what just
//! happened, or the search being typed.
//!
//! The keys are as many as fit, most used first, and never cut in half
//! (goal 2026-10-20-inbox-list-look, item 10): when some are left out the
//! line ends `? keys`, and `?` opens `/keys` on the inbox's own section.
use super::draw::Canvas;
use super::{InboxPane, Stage};
use crate::chatwidth::str_w;

type Keys = &'static [(&'static str, &'static str)];

const SETUP: Keys = &[
    ("1\u{2013}5", "open the console"),
    ("enter", "sign in"),
    ("esc", "close"),
];
const SIGNING_IN: Keys = &[("esc", "cancel")];
const LIST: Keys = &[
    ("enter", "read"),
    ("e", "archive"),
    ("d", "delete"),
    ("r", "reply"),
    ("/", "search"),
    ("u", "undo"),
    ("s", "star"),
    ("m", "read/unread"),
    ("a", "ask smith"),
    ("t", "todo"),
    ("R", "refresh"),
    ("L", "lines"),
    ("o", "gmail"),
];
const READER: Keys = &[
    ("esc", "back"),
    ("r", "reply"),
    ("e", "archive"),
    ("d", "delete"),
    ("z", "quotes"),
    ("i", "pictures"),
    ("v", "text"),
    ("w", "window"),
    ("a", "ask smith"),
    ("t", "todo"),
    ("m", "unread"),
    ("s", "star"),
    ("o", "gmail"),
];

/// What a left-out key's place says instead.
const MORE: &str = "? keys";
const SEP: &str = " \u{b7} ";

impl InboxPane {
    pub(crate) fn foot(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let last = cv.rows.saturating_sub(1);
        let room = usize::from(cv.cols.saturating_sub(2));
        let (text, fg) = match (&self.typing, self.note.is_empty()) {
            (Some(q), _) => (format!("search: {q}\u{258f}"), crate::palette::accent()),
            (None, false) => (self.note.clone(), t.bell),
            (None, true) => (fit(self.keys(), room), t.text_muted),
        };
        cv.text(last, 1, cv.cols, &text, fg, false);
    }

    fn keys(&self) -> Keys {
        match (&self.stage, &self.reading) {
            (Stage::Setup { .. }, _) => SETUP,
            (Stage::SigningIn(_), _) => SIGNING_IN,
            (Stage::Ready(_), Some(_)) => READER,
            (Stage::Ready(_), None) => LIST,
        }
    }
}

/// `keys` in at most `room` columns, whole: as many as fit with room left
/// for `? keys` after them, or all of them when all fit.
pub(crate) fn fit(keys: Keys, room: usize) -> String {
    let said: Vec<String> = keys.iter().map(|(k, what)| format!("{k} {what}")).collect();
    let all = said.join(SEP);
    if str_w(&all) <= room {
        return all;
    }
    let mut out = String::new();
    for s in &said {
        let next = match out.is_empty() {
            true => s.clone(),
            false => format!("{out}{SEP}{s}"),
        };
        if str_w(&next) + str_w(SEP) + str_w(MORE) > room {
            break;
        }
        out = next;
    }
    match out.is_empty() {
        true => MORE.into(),
        false => format!("{out}{SEP}{MORE}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{fit, LIST, READER};
    use crate::chatwidth::str_w;

    #[test]
    fn the_foot_keeps_whole_keys_most_used_first_and_points_at_the_rest() {
        for room in [20, 40, 58, 78, 118] {
            for keys in [LIST, READER] {
                let got = fit(keys, room);
                assert!(str_w(&got) <= room, "{room}: {got:?}");
                assert!(!got.contains('\u{2026}'), "{room}: {got:?}");
                assert!(got.ends_with("? keys"), "{room}: {got:?}");
                let first = format!("{} {}", keys[0].0, keys[0].1);
                assert!(got.starts_with(&first), "{room}: {got:?}");
            }
        }
        let wide = fit(LIST, 400);
        assert!(wide.ends_with("o gmail"), "everything fits: {wide:?}");
        assert_eq!(fit(LIST, 4), "? keys", "too narrow for any key");
    }
}
