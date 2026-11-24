//! The nav's Inbox section: the newest unread threads, two rows each — who wrote and how
//! long ago, then the subject — under an INBOX rule that carries the count. A click on a
//! thread opens it in the inbox pane; anywhere else on the panel opens the inbox
//! (`navinboxhit`). The rows come from the glance worker (`inboxglance`), which asks Gmail
//! every five minutes and mirrors the pane while it is open.
use crew_render::CellView;

use crate::inbox::wire::{sender_name, Thread};
use crate::inboxglance::Card;

/// Rows above the first thread: the rule.
pub(crate) const TOP: u16 = 1;
/// Rows a thread takes.
pub(crate) const PER: u16 = 2;

/// The threads a panel `rows` tall has room for.
pub(crate) fn fits(rows: u16) -> usize {
    usize::from(rows.saturating_sub(TOP) / PER)
}

/// The threads drawn for `card` and `list` in a panel `rows` tall: none when the panel says
/// something else instead (signed out, nothing unread, the first count still coming).
pub(crate) fn shown(card: &Card, list: &[Thread], rows: u16) -> usize {
    match card {
        Card::Off | Card::Count { unread: 0, .. } => 0,
        _ => list.len().min(fits(rows)),
    }
}

/// The thread on content row `row`, of `n` shown.
pub(crate) fn item_at(row: u16, n: usize) -> Option<usize> {
    let k = usize::from(row.checked_sub(TOP)? / PER);
    (k < n).then_some(k)
}

/// The section's cells for a panel `cols` × `rows`: `card` says whether there is a mailbox
/// at all, `list` what is unread in it, `hover` the thread under the pointer.
pub(crate) fn cells(
    card: &Card,
    list: &[Thread],
    hover: Option<usize>,
    now_ms: i64,
    (cols, rows): (u16, u16),
) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let key = match card {
        Card::Count { unread, .. } if *unread > 0 => format!("{unread} unread"),
        _ => String::new(),
    };
    let mut out = crate::boxdraw::section_header_key(
        "INBOX",
        &key,
        cols,
        t.border_normal,
        accent,
        t.text_muted,
        t.page_bg,
    );
    let max = cols.saturating_sub(1);
    let ind = crate::navtext::INDENT;
    let say = |out: &mut Vec<CellView>, row: u16, s: &str, fg| {
        crate::navtext::put_at(out, s, ind, row, max, fg);
    };
    match card {
        Card::Off => {
            say(&mut out, 1, "not signed in", t.text_muted);
            say(&mut out, 2, "click to sign in", accent);
            return out;
        }
        Card::Checking if list.is_empty() => {
            say(&mut out, 1, "checking\u{2026}", t.text_muted);
            return out;
        }
        Card::Problem(why) if list.is_empty() => {
            say(&mut out, 1, why, t.text_muted);
            return out;
        }
        Card::Count { unread: 0, .. } => {
            say(&mut out, 1, "nothing unread", t.text_muted);
            say(&mut out, 2, "click to open the inbox", t.text_muted);
            return out;
        }
        _ => {}
    }
    let shown = shown(card, list, rows);
    for (k, th) in list.iter().take(shown).enumerate() {
        let row = TOP + k as u16 * PER;
        let lit = hover == Some(k);
        let age = crate::inbox::age::short(th.at_ms, now_ms);
        // The age stands at the right edge; the sender takes what is left of the row.
        let age_col = max.saturating_sub(crate::chatwidth::str_w(&age) as u16);
        crate::navtext::put_at(&mut out, &age, age_col, row, max, t.text_muted);
        let from = out.len();
        let who = sender_name(&th.from);
        let who_fg = if lit { accent } else { t.ink };
        crate::navtext::put_at(&mut out, &who, ind, row, age_col.saturating_sub(1), who_fg);
        for c in &mut out[from..] {
            c.bold = true;
        }
        // The unread dot, in the gutter the indent leaves.
        crate::navtext::put_at(&mut out, "\u{25cf}", 1, row, ind, accent);
        let subject = match th.subject.trim() {
            "" => "(no subject)",
            s => s,
        };
        let sub_fg = if lit { accent } else { t.text_muted };
        say(&mut out, row + 1, subject, sub_fg);
    }
    // More unread than the panel shows: say how many the pane holds.
    let total = match card {
        Card::Count { unread, .. } => *unread as usize,
        _ => list.len(),
    };
    let row = TOP + shown as u16 * PER;
    if total > shown && row < rows {
        let more = format!("{} more in the inbox", total - shown);
        say(&mut out, row, &more, t.text_muted);
    }
    out
}

#[cfg(test)]
#[path = "navinbox_tests.rs"]
mod tests;
