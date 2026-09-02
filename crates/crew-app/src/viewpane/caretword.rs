//! A word at a time. Alt+Left/Right in every composer crew has hops runs of
//! non-blank characters ([`crate::todopane`]'s `cursor_word`); a document
//! window's caret hops the same runs, read off the rendered row.
use super::caret::{step, stops, Caret, Step};
use crate::chatbody::CardLine;

/// Every place the caret can stand on `line`, with the character there —
/// `None` at the row's end stop, past the last character.
fn places(line: &CardLine) -> Vec<(u16, Option<char>)> {
    let mut chars = std::collections::HashMap::new();
    let mut col = 0u16;
    for cell in line {
        let w = crate::chatwidth::char_w(cell.c) as u16;
        if w > 0 {
            chars.insert(col, cell.c);
            col += w;
        }
    }
    stops(line)
        .into_iter()
        .map(|(col, _)| (col, chars.get(&col).copied()))
        .collect()
}

/// The caret one word along: over any blanks first, then over the word. At
/// the row's end it is the ordinary step onto the next row — a word jump
/// that stops dead at a line end is one you have to finish with an arrow.
pub(crate) fn word(lines: &[CardLine], c: Caret, right: bool) -> Caret {
    let Some(line) = lines.get(c.row) else {
        return c;
    };
    let places = places(line);
    let Some(i) = places.iter().position(|&(col, _)| col == c.col) else {
        return c;
    };
    let blank = |j: usize| places[j].1.is_none_or(char::is_whitespace);
    let mut j = i;
    if right {
        while j + 1 < places.len() && blank(j) {
            j += 1;
        }
        while j + 1 < places.len() && !blank(j) {
            j += 1;
        }
    } else {
        while j > 0 && blank(j - 1) {
            j -= 1;
        }
        while j > 0 && !blank(j - 1) {
            j -= 1;
        }
    }
    if j == i {
        let dir = if right { Step::Right } else { Step::Left };
        return step(lines, c, dir);
    }
    let col = places[j].0;
    Caret {
        row: c.row,
        col,
        want: col,
    }
}

#[cfg(test)]
#[path = "caretword_tests.rs"]
mod tests;
