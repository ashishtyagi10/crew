//! The trunk a task's chained replies hang from.
//!
//! Replies to one task chain under its first card with tree connectors —
//! `├` while more follow, `└` on the last (`chatmsgs`). Their bodies start in
//! the column after the connector's, so the `├`s stood alone over text with
//! nothing joining them. A reply with more of the thread below it now draws
//! `│` in its body's indent column, top to bottom, and the tees become one
//! line. The indent cell is replaced rather than a column added: the body
//! keeps its width, its wrap and its columns, so nothing measured against
//! the drawn rows moves.
use crate::chatbody::{plain, CardLine};

const TRUNK: char = '\u{2502}';

/// Put the trunk in each of `body`'s indent cells (an empty row gets one).
pub(crate) fn hang(body: &mut [CardLine]) {
    let muted = crew_theme::theme().text_muted;
    for line in body {
        match line.first_mut() {
            Some(cell) if cell.c == ' ' && cell.bg.is_none() => {
                *cell = plain(TRUNK, muted, false);
            }
            Some(_) => {}
            None => line.push(plain(TRUNK, muted, false)),
        }
    }
}
