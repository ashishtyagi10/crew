//! The pane's foot: every key that works where you are, wrapped whole onto
//! as many lines as they take (the user, 2026-10-21: "all shortcut should
//! be visible"), each a keycap you can click with what it does quiet beside
//! it, in groups (`footkeys`); over them, on a line of its own, what just
//! happened or the search being typed. Only a pane too short to spare the
//! lines ends its last one `? keys`, and `?` opens `/keys` on the inbox's
//! own section.
use super::draw::Canvas;
use super::{InboxPane, Stage};
use crate::chatwidth::str_w;

pub(crate) use super::footkeys::{Key, Keys, GAP, LIST, READER};
use super::footkeys::{SEARCH, SETUP, SIGNING_IN};

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
        let said = match &self.typing {
            Some(_) => 1,
            None => said_lines(&self.note, room(cols)).len() as u16,
        };
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
        let (said, fg) = match &self.typing {
            Some(q) => (
                vec![format!("search: {q}\u{258f}")],
                crate::palette::accent(),
            ),
            None => (said_lines(&self.note, room(cv.cols)), t.bell),
        };
        for text in said {
            cv.text(row, 1, cv.cols, &text, fg, false);
            row += 1;
        }
        for line in lines(self.keys(), room(cv.cols), most(cv.rows)) {
            for (col, (k, what)) in placed(&line) {
                // The key on a keycap: the capsule inline code wears.
                let from = cv.out.len();
                cv.text(row, col, cv.cols, k, t.ink, true);
                for c in &mut cv.out[from..] {
                    (c.mark, c.bg) = (true, crate::chatink::code_bg());
                }
                let at = col + str_w(k) as u16 + 1;
                cv.text(row, at, cv.cols, what, t.text_muted, false);
            }
            row += 1;
        }
    }

    /// The key whose keycap or words are at `(row, col)` of the foot.
    pub(crate) fn foot_key_at(
        &self,
        cols: u16,
        rows: u16,
        row: u16,
        col: u16,
    ) -> Option<&'static str> {
        let said = match &self.typing {
            Some(_) => 1,
            None => said_lines(&self.note, room(cols)).len() as u16,
        };
        let first = rows.saturating_sub(self.foot_h(cols, rows)) + said;
        let lines = lines(self.keys(), room(cols), most(rows));
        let line = lines.get(usize::from(row.checked_sub(first)?))?;
        let (_, (k, _)) = placed(line)
            .into_iter()
            .find(|&(c, key)| (c..c + key_w(key) as u16).contains(&col))?;
        Some(k)
    }

    fn keys(&self) -> Keys {
        match (&self.stage, &self.reading) {
            (Stage::Setup { .. }, _) => SETUP,
            (Stage::SigningIn(_), _) => SIGNING_IN,
            (Stage::Ready(_), Some(_)) if self.compose.is_some() => self.compose_keys(),
            (Stage::Ready(_), Some(_)) => READER,
            // The list's keys type into the search while it is written.
            (Stage::Ready(_), None) if self.typing.is_some() => SEARCH,
            (Stage::Ready(_), None) => LIST,
        }
    }
}

/// The columns a line of keys may take: all but the margin each side.
fn room(cols: u16) -> usize {
    usize::from(cols.saturating_sub(2))
}

/// What just happened, wrapped at spaces onto up to [`SAID`] lines of `room`
/// columns. Cut at the pane's edge, a note lost its own end, and `u`'s ended
/// with what to press next (the user, 2026-10-21: "unable to unsubscribe,
/// however getting message at the bottom").
fn said_lines(note: &str, room: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in note.split(' ').filter(|w| !w.is_empty()) {
        match out.last_mut() {
            Some(line) if str_w(line) + 1 + str_w(word) <= room => {
                line.push(' ');
                line.push_str(word);
            }
            _ => out.push(word.to_string()),
        }
    }
    if out.len() > SAID {
        out.truncate(SAID);
        out[SAID - 1].push('\u{2026}');
    }
    out
}

/// The note's lines at most; past them the last ends in an ellipsis.
const SAID: usize = 3;

/// The key lines a pane `rows` tall can spare: a quarter of it, one at
/// least and [`LINES`] at most.
fn most(rows: u16) -> usize {
    usize::from((rows / 4).clamp(1, LINES))
}

/// A key's columns: its keycap, a space, what it does; a group's end none.
fn key_w(key: Key) -> usize {
    match key == GAP {
        true => 0,
        false => str_w(key.0) + 1 + str_w(key.1),
    }
}

fn width(line: &[Key]) -> usize {
    let keys: usize = line.iter().map(|&k| key_w(k)).sum();
    keys + str_w(SEP) * line.len().saturating_sub(1)
}

/// Each key on `line` with the column it starts at; a group's end leaves
/// its gap and no key.
fn placed(line: &[Key]) -> Vec<(u16, Key)> {
    let mut col = 1;
    let mut out = Vec::new();
    for &key in line {
        if key != GAP {
            out.push((col as u16, key));
        }
        col += key_w(key) + str_w(SEP);
    }
    out
}

/// `keys` wrapped whole into lines of at most `room` columns, in order. Past
/// `most` lines, the last ends with `? keys` in place of what is left out.
pub(crate) fn lines(keys: Keys, room: usize, most: usize) -> Vec<Vec<Key>> {
    let mut out: Vec<Vec<Key>> = Vec::new();
    for &key in keys {
        match out.last_mut() {
            Some(line) if width(line) + str_w(SEP) + width(&[key]) <= room => line.push(key),
            // A group that starts a line needs no gap before it.
            _ if key == GAP => {}
            _ => out.push(vec![key]),
        }
    }
    for line in &mut out {
        while line.last() == Some(&GAP) {
            line.pop();
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
