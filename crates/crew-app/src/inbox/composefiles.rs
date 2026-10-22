//! The reply box's row of files: each file of yours by name and size with a
//! `✕` to take it off, then `+ attach`, wrapping to more rows as they come.
//! One layout ([`spots`]) is what is drawn and what a click finds, and a
//! click does what the keys do (`composekeys`): a name opens the file, to
//! see it, `✕` takes it off, `+ attach` opens the chooser.
use super::compose::{Compose, On};
use super::composedraw::{text_w, X};
use super::composegeo::geo;
use super::draw::Canvas;
use super::InboxPane;
use crate::chatwidth::str_w;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum What {
    Open(usize),
    Remove(usize),
    Add,
}

/// One thing on the row: its row (from the first) and column (from the
/// words' left), its width, its words and a muted tail (a file's size).
pub(crate) struct Spot {
    pub row: u16,
    pub col: u16,
    pub w: u16,
    pub what: What,
    pub text: String,
    pub tail: String,
}

/// Between one file and the next.
const GAP: u16 = 3;

/// The row laid out in words `w` columns wide.
pub(crate) fn spots(c: &Compose, w: u16) -> Vec<Spot> {
    let room = w.saturating_sub(1);
    let mut out = Vec::new();
    let (mut row, mut col) = (0, 0);
    for (i, f) in c.files.iter().enumerate() {
        let tail = format!(" {}", super::save::size(f.size));
        let most = usize::from(room).saturating_sub(str_w(&tail) + 3);
        let text = clip(&f.name, most.max(4));
        let w = (str_w(&text) + str_w(&tail)) as u16;
        if col > 0 && col + w + 2 > room {
            (row, col) = (row + 1, 0);
        }
        out.push(Spot {
            row,
            col,
            w,
            what: What::Open(i),
            text,
            tail,
        });
        let x = Spot {
            row,
            col: col + w + 1,
            w: 1,
            what: What::Remove(i),
            text: "\u{2715}".into(),
            tail: String::new(),
        };
        out.push(x);
        col += w + 2 + GAP;
    }
    let text = match c.files.is_empty() {
        true => "+ attach a file",
        false => "+ attach",
    };
    let w = str_w(text) as u16;
    if col > 0 && col + w > room {
        (row, col) = (row + 1, 0);
    }
    let tail = match c.files.is_empty() {
        true => " \u{b7} or drop one on the pane".to_string(),
        false => String::new(),
    };
    out.push(Spot {
        row,
        col,
        w,
        what: What::Add,
        text: text.into(),
        tail,
    });
    out
}

/// The rows the row of files takes, in words `w` wide.
pub(crate) fn rows(c: &Compose, w: u16) -> u16 {
    spots(c, w).last().map_or(1, |s| s.row + 1)
}

/// `name` cut to `most` columns, an ellipsis where it was cut.
fn clip(name: &str, most: usize) -> String {
    if str_w(name) <= most {
        return name.to_string();
    }
    let mut out = String::new();
    for ch in name.chars() {
        if str_w(&out) + 2 > most {
            break;
        }
        out.push(ch);
    }
    out.push('\u{2026}');
    out
}

/// The row on `cv` from row `top`, in words `w` wide from column [`X`].
pub(crate) fn draw(cv: &mut Canvas, c: &Compose, top: u16, w: u16) {
    let end = X + w;
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let on = c.on == On::Files;
    let chosen = |w: What| {
        on && matches!(w, What::Open(i) | What::Remove(i) if i == c.pick)
            || on && w == What::Add && c.pick >= c.files.len()
    };
    for s in spots(c, w) {
        let (row, col) = (top + s.row, X + s.col);
        let lit = chosen(s.what);
        let ink = match (s.what, lit) {
            (_, true) => accent,
            (What::Open(_), false) => t.ink,
            (What::Remove(_), false) => t.text_muted,
            (What::Add, false) => accent,
        };
        cv.text(row, col, end, &s.text, ink, lit);
        let after = col + str_w(&s.text) as u16;
        cv.text(row, after, end, &s.tail, t.text_muted, false);
    }
}

impl InboxPane {
    /// What on the box's row of files is at the pane's `(row, col)`, if
    /// anything: what a click does there, and where the pointer is a hand.
    pub(crate) fn compose_spot_at(&self, cols: u16, rows: u16, row: u16, col: u16) -> Option<What> {
        let b = self.cards(cols, rows).reply?;
        let ((row, col), c) = (b.local(row, col)?, self.compose.as_ref()?);
        let w = text_w(b.w) as u16;
        let f = geo(c, 0, b.h, w).files?;
        let s = spots(c, w).into_iter().find(|s| {
            row.checked_sub(f) == Some(s.row) && (X + s.col..X + s.col + s.w).contains(&col)
        })?;
        Some(s.what)
    }

    /// A click on the pane's `(row, col)`: the caret to the box's line
    /// there, or a file's open or `✕`, or `+ attach`. True when it was on
    /// the box.
    pub(crate) fn compose_click(&mut self, cols: u16, rows: u16, row: u16, col: u16) -> bool {
        let hit = self.compose_spot_at(cols, rows, row, col);
        let Some(b) = self.cards(cols, rows).reply else {
            return false;
        };
        let (Some((row, col)), Some(c)) = (b.local(row, col), self.compose.as_mut()) else {
            return false;
        };
        let w = text_w(b.w) as u16;
        let g = geo(c, 0, b.h, w);
        let files = g
            .files
            .filter(|&f| (f..f + self::rows(c, w)).contains(&row));
        c.on = match row {
            _ if files.is_some() => On::Files,
            r if r <= g.to => On::To,
            r if Some(r) == g.cc => On::Cc,
            _ => On::Body,
        };
        super::composegeo::caret_at(c, &g, w, (row, col));
        match hit {
            Some(What::Open(i)) => {
                c.pick = i;
                self.open_attached(i);
            }
            Some(What::Remove(i)) => self.unattach(i),
            Some(What::Add) => {
                c.pick = c.files.len();
                self.choose_files();
            }
            None => {}
        }
        true
    }
}
