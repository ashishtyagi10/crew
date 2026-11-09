//! How far a receipt's change reaches, on its head's first row.
//!
//! The broker folds ` · reaches 6 callers in 3 files` into the head line
//! (`broker::receiptreach`); unread, it stuck to the last path, which then
//! read `src/lib.rs · reaches 2 callers in 2 files` as one file name. It is
//! read off here and drawn at the end of the head's first row — the counts
//! in the card's ink, the words muted — so it costs no row where it fits.
use crate::chatbody::{CardLine, Color};
use crate::chatreceiptcells::push;
use crate::chatwidth::str_w;

/// `rest` without its trailing reach, and the reach as `(callers, files)`.
pub(crate) fn split(rest: &str) -> (&str, Option<(usize, usize)>) {
    let Some((list, said)) = rest.rsplit_once(" \u{b7} reaches ") else {
        return (rest, None);
    };
    let w: Vec<&str> = said.split(' ').collect();
    match w.as_slice() {
        [n, _, "in", f, _] => match (n.parse(), f.parse()) {
            (Ok(n), Ok(f)) => (list, Some((n, f))),
            _ => (rest, None),
        },
        _ => (rest, None),
    }
}

/// The reach at the end of the head's first row when it fits there; at a
/// narrow tile its short form, ` · 6 callers`, which still costs no row; on
/// a row of its own under the head only when even that does not fit.
pub(crate) fn reach_on(out: &mut Vec<CardLine>, (n, f): (usize, usize), cols: usize, fg: Color) {
    let muted = crew_theme::theme().text_muted;
    let s = |k: usize| if k == 1 { "" } else { "s" };
    let lead = (" \u{b7} reaches ".to_string(), muted);
    let count = (format!("{n} caller{}", s(n)), fg);
    let files = (format!(" in {f} file{}", s(f)), muted);
    let full = [lead, count.clone(), files];
    let short = [(" \u{b7} ".to_string(), muted), count];
    let w = |parts: &[(String, Color)]| parts.iter().map(|(t, _)| str_w(t)).sum::<usize>();
    let room = out.first().map_or(0, |r| cols.saturating_sub(r.len()));
    let (row, parts): (&mut CardLine, &[(String, Color)]) = match out.first_mut() {
        Some(first) if w(&full) < room => (first, &full),
        Some(first) if w(&short) < room => (first, &short),
        _ => {
            out.push(vec![crate::chatbody::plain(' ', fg, false)]);
            (out.last_mut().expect("just pushed"), &full)
        }
    };
    for (text, ink) in parts {
        push(row, text, *ink);
    }
}

#[cfg(test)]
#[path = "chatreceiptreach_tests.rs"]
mod tests;
