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

#[cfg(test)]
#[path = "composegeo_tests.rs"]
mod tests;
