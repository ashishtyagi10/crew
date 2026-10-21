//! The pane's foot: every key that works where you are, wrapped whole onto
//! as many lines as they take (the user, 2026-10-21: "all shortcut should
//! be visible"), each key bright and what it does quiet; over them, on a
//! line of its own, what just happened or the search being typed. Only a
//! pane too short to spare the lines ends its last one `? keys`, and `?`
//! opens `/keys` on the inbox's own section.
use super::draw::Canvas;
use super::{InboxPane, Stage};
use crate::chatwidth::str_w;

type Key = (&'static str, &'static str);
type Keys = &'static [Key];

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
    ("z", "undo"),
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
    ("z", "undo"),
    ("u", "unsubscribe"),
    ("q", "quoted replies"),
    ("i", "pictures"),
    ("v", "text"),
    ("w", "window"),
    ("a", "ask smith"),
    ("t", "todo"),
    ("m", "unread"),
    ("s", "star"),
    ("o", "gmail"),
];

/// Between two keys on a line.
const SEP: &str = "  ";
/// What stands for the keys a short pane leaves out.
const MORE: Key = ("?", "keys");
/// The foot's key lines at most, however many keys there are.
const LINES: u16 = 4;

impl InboxPane {
    /// The rows the foot takes in a pane `cols` by `rows`: the note's line
    /// when there is one, and the keys' lines. The pane draws above them.
    pub(crate) fn foot_h(&self, cols: u16, rows: u16) -> u16 {
        let said = u16::from(self.typing.is_some() || !self.note.is_empty());
        (said + lines(self.keys(), room(cols), most(rows)).len() as u16).min(rows)
    }

    /// The rows the list, the reader or the setup page draw in.
    pub(crate) fn body_rows(&self, cols: u16, rows: u16) -> u16 {
        rows - self.foot_h(cols, rows)
    }

    /// The foot, on the last [`Self::foot_h`] rows of `cv`.
    pub(crate) fn foot(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let mut row = cv.rows.saturating_sub(self.foot_h(cv.cols, cv.rows));
        let said = match &self.typing {
            Some(q) => Some((format!("search: {q}\u{258f}"), crate::palette::accent())),
            None if !self.note.is_empty() => Some((self.note.clone(), t.bell)),
            None => None,
        };
        if let Some((text, fg)) = said {
            cv.text(row, 1, cv.cols, &text, fg, false);
            row += 1;
        }
        for line in lines(self.keys(), room(cv.cols), most(cv.rows)) {
            let mut col = 1;
            for (k, what) in line {
                cv.text(row, col, cv.cols, k, t.ink, false);
                let at = col + str_w(k) as u16 + 1;
                cv.text(row, at, cv.cols, what, t.text_muted, false);
                col = at + (str_w(what) + str_w(SEP)) as u16;
            }
            row += 1;
        }
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

/// The columns a line of keys may take: all but the margin each side.
fn room(cols: u16) -> usize {
    usize::from(cols.saturating_sub(2))
}

/// The key lines a pane `rows` tall can spare: a quarter of it, one at
/// least and [`LINES`] at most.
fn most(rows: u16) -> usize {
    usize::from((rows / 4).clamp(1, LINES))
}

fn width(line: &[Key]) -> usize {
    let keys: usize = line.iter().map(|(k, w)| str_w(k) + 1 + str_w(w)).sum();
    keys + str_w(SEP) * line.len().saturating_sub(1)
}

/// `keys` wrapped whole into lines of at most `room` columns, in order. Past
/// `most` lines, the last ends with `? keys` in place of what is left out.
pub(crate) fn lines(keys: Keys, room: usize, most: usize) -> Vec<Vec<Key>> {
    let mut out: Vec<Vec<Key>> = Vec::new();
    for &key in keys {
        match out.last_mut() {
            Some(line) if width(line) + str_w(SEP) + width(&[key]) <= room => line.push(key),
            _ => out.push(vec![key]),
        }
    }
    if out.len() > most {
        out.truncate(most);
        if let Some(last) = out.last_mut() {
            while !last.is_empty() && width(last) + str_w(SEP) + width(&[MORE]) > room {
                last.pop();
            }
            last.push(MORE);
        }
    }
    out
}

#[cfg(test)]
#[path = "foot_tests.rs"]
mod tests;
