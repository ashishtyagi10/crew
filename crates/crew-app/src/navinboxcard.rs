//! The INBOX card in the nav's glance slot: how many threads are unread,
//! and who wrote the newest one about what — the mail a glance should
//! catch. It shows only when signed in (`inboxglance`), stands under
//! WEATHER, and is the first card dropped when the nav is short. A click
//! opens `/inbox`.
use crew_render::CellView;

use crate::inboxglance::Card;
use crate::palette::accent;

/// Rows for `card`: rule, the count, the newest unread (when there is one),
/// and a one-row gap; nothing when signed out.
pub(crate) fn block(card: &Card) -> u16 {
    match card {
        Card::Off => 0,
        Card::Count {
            newest: Some(_),
            unread,
        } if *unread > 0 => 4,
        _ => 3,
    }
}

pub(crate) fn cells(card: &Card, cols: u16) -> Vec<CellView> {
    if cols < 8 || *card == Card::Off {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut out = crate::boxdraw::section_header_key(
        "INBOX",
        "",
        cols,
        t.border_normal,
        accent(),
        t.text_muted,
        t.page_bg,
    );
    let max_col = cols.saturating_sub(1);
    let col = crate::navtext::INDENT;
    let line = |out: &mut Vec<CellView>, row: u16, s: &str, fg, bold: bool| {
        let from = out.len();
        crate::navtext::put_at(out, s, col, row, max_col, fg);
        for c in &mut out[from..] {
            c.bold = bold;
        }
    };
    match card {
        Card::Off => {}
        Card::Checking => line(&mut out, 1, "checking\u{2026}", t.text_muted, false),
        Card::Problem(why) => line(&mut out, 1, why, t.text_muted, false),
        Card::Count { unread: 0, .. } => line(&mut out, 1, "nothing unread", t.text_muted, false),
        Card::Count { unread, newest } => {
            let threads = crate::wording::count(*unread as usize, "unread thread");
            line(&mut out, 1, &format!("\u{25cf} {threads}"), accent(), true);
            if let Some((who, subject)) = newest {
                let said = match subject.is_empty() {
                    true => who.clone(),
                    false => format!("{who} \u{b7} {subject}"),
                };
                line(&mut out, 2, &said, t.ink, false);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "navinboxcard_tests.rs"]
mod tests;
