//! Where the reply box's parts sit, row by row. A short pane gives the box
//! few rows (the mail's card is always the taller), so what is not in use
//! gives way first: the row of air under the addresses, then an empty `cc`
//! line the caret is not on, then an empty row of files, and only then a
//! filled one. The `to` line, the place the caret is on and a row for the
//! words show first, and nothing is drawn on the box's edge.
use super::compose::{Compose, On};

pub(crate) struct Geo {
    /// The `to` line's row, and the `cc` line's when it shows.
    pub to: u16,
    pub cc: Option<u16>,
    /// The words' first row, and how many they have.
    pub words: u16,
    pub room: u16,
    /// The row of files' first row, when it shows.
    pub files: Option<u16>,
}

/// The parts of a box whose top edge is at `top`, `h` rows tall, its words
/// `w` wide.
pub(crate) fn geo(c: &Compose, top: u16, h: u16, w: u16) -> Geo {
    let inside = h.saturating_sub(2);
    let n = super::composefiles::rows(c, w);
    let carried = u16::from(c.forward.is_some());
    // The `to` line and a row of words, and whatever else still shows.
    let need = |cc: bool, air: bool, files: bool| {
        2 + u16::from(cc) + u16::from(air) + if files { n } else { 0 } + carried
    };
    let (mut cc, mut air, mut files) = (true, true, true);
    air &= need(cc, air, files) <= inside;
    cc &= need(cc, air, files) <= inside || c.on == On::Cc || !c.cc.text.is_empty();
    files &= need(cc, air, files) <= inside || c.on == On::Files || !c.files.is_empty();
    // Still too many: the words win over a line the caret is not on.
    cc &= need(cc, air, files) <= inside || c.on == On::Cc;
    files &= need(cc, air, files) <= inside || c.on == On::Files;
    let to = top + 1;
    let words = to + 1 + u16::from(cc) + u16::from(air);
    // The rows the files and the carried mail take, above the bottom edge.
    let under = (top + h).saturating_sub(1 + carried);
    let first = under.saturating_sub(n);
    Geo {
        to,
        cc: cc.then_some(to + 1),
        words,
        room: if files { first } else { under }.saturating_sub(words),
        files: files.then_some(first),
    }
}

/// A click at the box's own `(row, col)`, on the place `c.on` already
/// names: the caret to the letter there. In the words, the last letter at or
/// before the column on the row clicked (below the words, their end); on an
/// address line shown whole, the letter under the pointer.
pub(crate) fn caret_at(c: &mut Compose, g: &Geo, w: u16, (row, col): (u16, u16)) {
    use super::composedraw::{place, X};
    let w = usize::from(w);
    match c.on {
        On::Body if (g.words..g.words + g.room).contains(&row) => {
            let text = &c.body.text;
            let (now, _) = place(text, c.body.at, w);
            let first = now.saturating_sub(usize::from(g.room).saturating_sub(1));
            let (line, col) = (
                first + usize::from(row - g.words),
                usize::from(col.saturating_sub(X)),
            );
            let n = text.chars().count();
            let at = (0..=n).filter(|&i| {
                let (r, k) = place(text, i, w);
                r == line && k <= col
            });
            c.body.at = at.last().unwrap_or(n);
        }
        On::To | On::Cc => {
            let room = w.saturating_sub(usize::from(super::composehead::VALUE - X));
            let f = c.field();
            if crate::chatwidth::str_w(&f.text) < room {
                let col = usize::from(col.saturating_sub(super::composehead::VALUE));
                let mut x = 0;
                f.at = f
                    .text
                    .chars()
                    .take_while(|&ch| {
                        x += crate::chatwidth::char_w(ch);
                        x <= col
                    })
                    .count();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "composegeo_tests.rs"]
mod tests;
