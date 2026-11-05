//! A receipt's head, drawn: one row per file, its `+a −d` in the diff's own
//! inks and a small bar, under a lead row only when there is more than one.
//!
//! The broker's head is one line of text, `task #3: 1 file changed (+4 −1):
//! ~ crates/…/lib.rs` (`broker::receipt`), and it stays that — it is parsed
//! by the pane and read back by the tests. Printed, it wrapped at a quarter
//! tile as `… (+4 −1): ~` / `crates/crew-theme/src/lib.rs`, the marker on
//! one row and its path on the next; its counts were in the card's grey,
//! several files ran together on one line, and `task #3:` named a number
//! the user never sees anywhere else. A file is now a row that never wraps:
//! the path gives up its LEFT end (`…/src/lib.rs`), never its marker.
use crate::chatbody::{plain, CardLine, Color};
use crate::chatreceiptparse::{parse, per_file};
use crate::chatwidth::{char_w, str_w};
use crate::md::syntax::Token;

/// How many files get a row; the rest are counted on one.
pub(crate) const SHOWN: usize = 3;
/// The bar's width at the largest file's change, in cells.
pub(crate) const BAR: usize = 5;
/// The bar's cell: the heavy rule the tool strip uses, drawn whole.
const CELL: char = '\u{2501}';

fn ink(token: Token) -> Color {
    crate::chatink::token_fg(token)
}

fn push(row: &mut CardLine, s: &str, fg: Color) {
    row.extend(s.chars().map(|c| plain(c, fg, false)));
}

/// `+a −d`, each in its diff ink.
fn push_counts(row: &mut CardLine, (a, d): (usize, usize)) {
    push(row, &format!("+{a}"), ink(Token::Added));
    push(row, &format!(" \u{2212}{d}"), ink(Token::Removed));
}

/// The bar: as long as this file's change against the largest's, split
/// between the added and removed inks in their proportion.
fn push_bar(row: &mut CardLine, (a, d): (usize, usize), max: usize) {
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
fn tail_fit(path: &str, w: usize) -> String {
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

/// The drawn head of the receipt whose whole text is `text`, `cols` wide;
/// `None` when its first line does not parse.
pub(crate) fn rows(text: &str, cols: usize, fg: Color) -> Option<Vec<CardLine>> {
    let head = parse(text.lines().next()?)?;
    let width = cols.saturating_sub(2).max(1);
    let mut out: Vec<CardLine> = Vec::new();
    if head.files > 1 || head.ending.is_some() {
        let mut row = vec![plain(' ', fg, false)];
        let s = if head.files == 1 { "" } else { "s" };
        let lead = format!("{} file{s} changed", head.files);
        push(
            &mut row,
            &head
                .ending
                .map_or(lead.clone(), |e| format!("{e} \u{b7} {lead}")),
            fg,
        );
        if let Some(t) = head.total.filter(|_| head.files > 1) {
            push(&mut row, "  ", fg);
            push_counts(&mut row, t);
        }
        out.push(row.into_iter().take(width + 1).collect());
    }
    let diff = per_file(text);
    let shown: Vec<_> = head.named.iter().take(SHOWN).collect();
    let counts: Vec<Option<(usize, usize)>> = shown
        .iter()
        .map(|(_, p)| match head.files {
            1 => head.total,
            _ => diff.iter().find(|f| f.0 == *p).map(|f| (f.1, f.2)),
        })
        .collect();
    let cw = counts
        .iter()
        .flatten()
        .map(|(a, d)| str_w(&format!("+{a} \u{2212}{d}")) + 2);
    let cw = cw.max().unwrap_or(0);
    let max = counts
        .iter()
        .flatten()
        .map(|(a, d)| a + d)
        .max()
        .unwrap_or(0);
    let pw = shown.iter().map(|(_, p)| str_w(p)).max().unwrap_or(0);
    let avail = width.saturating_sub(2); // the marker and its space
    let bar = cw > 0 && pw + cw + 2 + BAR <= avail;
    let pw = pw.min(avail.saturating_sub(cw).max(4));
    for ((mark, path), n) in shown.iter().zip(&counts) {
        let mfg = match *mark {
            "+" => ink(Token::Added),
            "\u{2212}" => ink(Token::Removed),
            _ => fg,
        };
        let mut row = vec![plain(' ', fg, false)];
        push(&mut row, mark, mfg);
        let p = tail_fit(path, pw);
        push(&mut row, &format!(" {p}{}", " ".repeat(pw - str_w(&p))), fg);
        if let Some(n) = *n {
            push(&mut row, "  ", fg);
            let at = row.len();
            push_counts(&mut row, n);
            if bar {
                let pad = cw - (row.len() - at);
                push(&mut row, &" ".repeat(pad), fg);
                push_bar(&mut row, n, max);
            }
        }
        out.push(row.into_iter().take(width + 1).collect());
    }
    if let Some(more) = head.files.checked_sub(shown.len()).filter(|n| *n > 0) {
        let muted = crew_theme::theme().text_muted;
        let s = if more == 1 { "" } else { "s" };
        let mut row = vec![plain(' ', fg, false)];
        push(&mut row, &format!("+{more} more file{s}"), muted);
        out.push(row);
    }
    if let Some(h) = head.hint {
        out.extend(crate::chatbody::body_lines(h, cols, fg, false));
    }
    Some(out)
}

/// A card's body: a receipt's head drawn ([`rows`]) over the rest of
/// `shown` laid out as markdown; any other card (or the source view, or a
/// head that does not parse) laid out whole, as before.
pub(crate) fn body(
    m: &crate::chatlayout::Message,
    shown: &str,
    cols: usize,
    fg: Color,
    view: crate::chatmsgs::View<'_>,
) -> Vec<CardLine> {
    let whole = || crate::chatbody::body_lines_at(shown, cols, fg, view.source, view.cwd);
    if view.source || !crate::chatreceipt::is_receipt(m) {
        return whole();
    }
    let Some(mut out) = rows(&m.text, cols, fg) else {
        return whole();
    };
    if let Some((_, rest)) = shown.split_once('\n').filter(|(_, r)| !r.trim().is_empty()) {
        out.extend(crate::chatbody::body_lines_at(
            rest, cols, fg, false, view.cwd,
        ));
    }
    out
}

#[cfg(test)]
#[path = "chatreceipthead_tests.rs"]
mod tests;
