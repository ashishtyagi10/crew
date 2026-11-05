//! The cells a receipt's head is drawn from (`chatreceipthead`): counts in
//! the diff's own inks, the per-file bar, and a path that gives up its LEFT
//! end, never its marker, when the row is narrow.
use crate::chatbody::{plain, CardLine, Color};
use crate::chatwidth::{char_w, str_w};
use crate::md::syntax::Token;

/// The bar's width at the largest file's change, in cells.
pub(crate) const BAR: usize = 5;
/// The bar's cell: the heavy rule the tool strip uses, drawn whole.
pub(crate) const CELL: char = '\u{2501}';

pub(crate) fn ink(token: Token) -> Color {
    crate::chatink::token_fg(token)
}

pub(crate) fn push(row: &mut CardLine, s: &str, fg: Color) {
    row.extend(s.chars().map(|c| plain(c, fg, false)));
}

/// `+a −d`, each in its diff ink.
pub(crate) fn push_counts(row: &mut CardLine, (a, d): (usize, usize)) {
    push(row, &format!("+{a}"), ink(Token::Added));
    push(row, &format!(" \u{2212}{d}"), ink(Token::Removed));
}

/// The bar: as long as this file's change against the largest's, split
/// between the added and removed inks in their proportion.
pub(crate) fn push_bar(row: &mut CardLine, (a, d): (usize, usize), max: usize) {
    let t = a + d;
    if t == 0 || max == 0 {
        return;
    }
    let cells = (t * BAR).div_ceil(max).clamp(1, BAR);
    let mut add = (cells * a + t / 2) / t;
    if a > 0 && add == 0 {
        add = 1;
    }
    if d > 0 && add == cells && cells > 1 {
        add = cells - 1;
    }
    row.extend(std::iter::repeat_n(
        plain(CELL, ink(Token::Added), false),
        add,
    ));
    let del = ink(Token::Removed);
    row.extend(std::iter::repeat_n(plain(CELL, del, false), cells - add));
}

/// `path` in at most `w` cells, its left end given up first: the file's
/// name is what a reader looks for.
pub(crate) fn tail_fit(path: &str, w: usize) -> String {
    if str_w(path) <= w {
        return path.to_string();
    }
    let mut kept: Vec<char> = Vec::new();
    let mut used = 1; // the ellipsis
    for c in path.chars().rev() {
        if used + char_w(c) > w {
            break;
        }
        used += char_w(c);
        kept.push(c);
    }
    std::iter::once('\u{2026}')
        .chain(kept.into_iter().rev())
        .collect()
}
