//! The PANES list laid out as cards ([`crate::navpanecard`]): the same order
//! and the same lines [`crate::tablines`]'s map gives — a card's two, a gap
//! between the cards of one group, a tab card's edges around its group — so
//! the row a click lands on is the row that lit up.
use crew_render::CellView;

use crate::navpanecard::card_cells;
use crate::panelist::PaneRow;
use crate::tabnav::TabBlock;

/// Whether this list draws as cards: every row carries one.
pub(crate) fn all_cards(rows: &[PaneRow]) -> bool {
    !rows.is_empty() && rows.iter().all(|r| r.card.is_some())
}

/// Whether a tab list draws as cards: every pane in every tab carries one.
pub(crate) fn tabs_cards(blocks: &[TabBlock]) -> bool {
    blocks.iter().any(|b| !b.rows.is_empty())
        && blocks.iter().all(|b| b.rows.iter().all(|r| r.card.is_some()))
}

/// One group's cards from `row`, in a list `cols` wide shifted `dx` columns
/// in, stopping before `end`. Returns the row after the last line drawn and
/// how many cards were drawn whole.
fn group(
    out: &mut Vec<CellView>,
    rows: &[PaneRow],
    (cols, dx): (u16, u16),
    mut row: u16,
    end: u16,
    sides: Option<(u16, (u8, u8, u8))>,
) -> (u16, usize) {
    let mut shown = 0;
    for (k, r) in rows.iter().enumerate() {
        let Some(c) = &r.card else { continue };
        if k > 0 {
            if row >= end {
                break;
            }
            if let Some((w, frame)) = sides {
                out.extend(crate::tabcard::sides(w, row, frame));
            }
            row += 1;
        }
        if row + 1 >= end {
            break;
        }
        out.extend(card_cells(r, c, cols, row).into_iter().map(|mut c| {
            c.col += dx;
            c
        }));
        if let Some((w, frame)) = sides {
            out.extend(crate::tabcard::sides(w, row, frame));
            out.extend(crate::tabcard::sides(w, row + 1, frame));
        }
        row += 2;
        shown += 1;
    }
    (row, shown)
}

/// The flat list: the PANES rule, then a card per pane.
pub(crate) fn flat(rows: &[PaneRow], cols: u16, limit: usize) -> Vec<CellView> {
    let end = end_row(limit);
    let mut body = Vec::new();
    let (_, shown) = group(&mut body, rows, (cols, 0), 1, end, None);
    let key = crate::navtext::depth_key(shown, rows.len());
    let mut out = rule(&key, cols);
    out.extend(body);
    out
}

/// One tab card per tab, its panes' cards inside it.
pub(crate) fn tabs(blocks: &[TabBlock], cols: u16, limit: usize) -> Vec<CellView> {
    let t = crew_theme::theme();
    let accent = crate::palette::accent();
    let end = end_row(limit);
    let (mut body, mut row, mut shown) = (Vec::new(), 1u16, 0);
    for b in blocks {
        let frame = match (b.active, b.needs_you()) {
            (true, _) => accent,
            (false, true) => t.bell,
            (false, false) => t.border_normal,
        };
        if row >= end {
            break;
        }
        body.extend(crate::tabcard::top(b, cols, row, frame));
        let inner = (cols.saturating_sub(3), 1);
        let (next, n) = group(&mut body, &b.rows, inner, row + 1, end, Some((cols, frame)));
        (row, shown) = (next, shown + n);
        if row < end {
            body.extend(crate::tabcard::bottom(cols, row, frame));
            row += 1;
        }
    }
    let total: usize = blocks.iter().map(|b| b.rows.len()).sum();
    let key = format!(
        "{} tabs \u{b7} {}",
        blocks.len(),
        crate::navtext::depth_key(shown, total)
    );
    let mut out = rule(&key, cols);
    out.extend(body);
    out
}

/// The row past the last one `limit` lines under the rule allow.
fn end_row(limit: usize) -> u16 {
    1 + limit.min(usize::from(u16::MAX - 1)) as u16
}

fn rule(key: &str, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    crate::boxdraw::section_header_key(
        "PANES",
        key,
        cols,
        t.border_normal,
        crate::palette::accent(),
        t.text_muted,
        t.page_bg,
    )
}

#[cfg(test)]
#[path = "navcardlist_tests.rs"]
mod tests;
