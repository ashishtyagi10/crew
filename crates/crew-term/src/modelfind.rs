//! Every match of a typed query across the whole buffer, for the pane's find
//! bar (`crew-app`'s `termfind`).
//!
//! alacritty's own regex search does the walking: it reads the grid's cells
//! (so a word a soft wrap split is still one match, and a full-width glyph's
//! parked blank is not a column), and it applies smart case itself, folding
//! unless the query holds an uppercase letter. What it lacks is a literal
//! mode, so the query is escaped into a pattern that can only match itself.
//! The walk goes from the bottom up: the newest match is first, and a query
//! that matches a flood of lines keeps the newest [`FIND_CAP`] of them.
use super::*;
use alacritty_terminal::index::Direction;
use alacritty_terminal::term::search::{RegexIter, RegexSearch};

/// How many matches a search keeps: a query of one letter over a full
/// scrollback has hundreds of thousands, and nobody steps through them.
pub const FIND_CAP: usize = 5000;

/// Where one match sits, in grid lines (`0` is the live screen's top row,
/// negative lines are scrollback) and columns, both ends inclusive. A match
/// a soft wrap split runs over more than one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FindSpan {
    pub line: i32,
    pub col: u16,
    pub end_line: i32,
    pub end_col: u16,
}

/// `query` as a regex that matches exactly that text: every ASCII
/// punctuation mark becomes a `\x{..}` escape, so no character of what was
/// typed is read as syntax. (Backslashing them is not enough: `\<` and `\>`
/// are word boundaries, so `Vec<u8>` would never match.)
fn literal(query: &str) -> String {
    let mut out = String::with_capacity(query.len() * 2);
    for c in query.chars() {
        match c.is_ascii_punctuation() {
            true => out.push_str(&format!("\\x{{{:x}}}", c as u32)),
            false => out.push(c),
        }
    }
    out
}

impl TermCore {
    /// The matches of `query` in the buffer, newest first (see the module
    /// docs). Empty for an empty query.
    pub(crate) fn find_all(&self, query: &str) -> Vec<FindSpan> {
        if query.is_empty() {
            return Vec::new();
        }
        let Ok(mut re) = RegexSearch::new(&literal(query)) else {
            return Vec::new();
        };
        let g = self.term.grid();
        let last = Column(g.columns().saturating_sub(1));
        let start = Point::new(Line(g.screen_lines() as i32 - 1), last);
        let end = Point::new(Line(-(g.history_size() as i32)), Column(0));
        RegexIter::new(start, end, Direction::Left, &self.term, &mut re)
            .take(FIND_CAP)
            .map(|m| FindSpan {
                line: m.start().line.0,
                col: m.start().column.0 as u16,
                end_line: m.end().line.0,
                end_col: m.end().column.0 as u16,
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "modelfind_tests.rs"]
mod modelfind_tests;
