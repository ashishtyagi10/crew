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
#[cfg(test)]
use crate::chatreceiptcells::CELL;
use crate::chatreceiptcells::{ink, push, push_bar, push_counts, tail_fit, BAR};
use crate::chatreceiptparse::{parse, per_file};
use crate::chatwidth::str_w;
use crate::md::syntax::Token;

/// How many files get a row; the rest are counted on the lead row.
pub(crate) const SHOWN: usize = 3;

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
        // The files with no row of their own are counted HERE, not on a
        // row of their own under the list: v0.29.77's `+2 more files` row
        // made a five-file head six rows.
        if let Some(more) = head.files.checked_sub(SHOWN).filter(|n| *n > 0) {
            let muted = crew_theme::theme().text_muted;
            push(&mut row, &format!(" \u{b7} {more} more"), muted);
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
    if let Some(reach) = head.reach {
        crate::chatreceiptreach::reach_on(&mut out, reach, cols, fg);
    }
    if let Some(h) = head.hint.map(tip) {
        tip_on(&mut out, h, cols, fg);
    }
    Some(out)
}

/// The broker's once-a-session hint as the pane says it. Its `/diff shows
/// them` is dropped: the card under this head holds the diff itself, one
/// click on its `▸ diff` row away, so the lesson is the card's own job.
fn tip(hint: &str) -> &str {
    hint.strip_prefix("/diff shows them; ").unwrap_or(hint)
}

/// `tip` muted at the end of the head's first row when it fits there, so
/// it costs no row; on a row of its own under the head when it does not.
fn tip_on(out: &mut Vec<CardLine>, tip: &str, cols: usize, fg: Color) {
    let muted = crew_theme::theme().text_muted;
    let suffix = format!(" \u{b7} {tip}");
    match out.first_mut() {
        Some(first) if first.len() + str_w(&suffix) < cols => push(first, &suffix, muted),
        _ => out.extend(crate::chatbody::body_lines(tip, cols, fg, false)),
    }
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
