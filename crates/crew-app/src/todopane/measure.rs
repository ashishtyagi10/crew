//! How TALL: row heights, the header, the popup, the list, and which rows
//! start a new day group.
//!
//! Its sibling [`super::fitline`] answers how WIDE — where a title wraps and
//! what fits on one line beside its chips.
//!
//! Split from [`super::render`] for the line cap, along the line between
//! working out where something goes and putting cells there. Both the counting
//! and the drawing pass read these, so they cannot disagree about the layout.
pub(crate) use super::fitline::*;
use super::item::TodoItem;
use super::{composer, duedate, TodoPane};

/// Cap on visible popup rows (incl. its 2 border rows).
pub(crate) const POPUP_MAX: u16 = 8;

/// See [`content`]. Wide enough for a real task, a project and a due stamp
/// with air between them; narrow enough that they stay one row.
pub(crate) const MAX_LIST_W: u16 = 92;

/// Where the done button sits on the header row: `(start, end)` columns, or
/// `None` when there is no button. Render and hit-test both read this, so
/// the button can't drift out from under the click.
pub(crate) fn done_chip_zone(p: &TodoPane, cols: u16) -> Option<(u16, u16)> {
    let chip = done_chip(p)?;
    let end = cols.saturating_sub(1);
    let start = end.saturating_sub(crate::chatwidth::str_w(&chip) as u16);
    (start > TITLE_COL).then_some((start, end))
}

/// The dim info row above the list: the `@project` filter's summary, the
/// done button, or both.
pub(crate) fn header_h(p: &TodoPane, cols: u16) -> u16 {
    // A button too wide for the pane isn't drawn, so it must not reserve the
    // row either — a narrow pane keeps every line for the list.
    u16::from(p.filter.is_some() || done_chip_zone(p, cols).is_some())
}

/// Rows the open tag popup occupies (0 when closed or the pane is short).
pub(crate) fn popup_h(p: &TodoPane, rows: u16) -> u16 {
    match &p.tagmenu {
        Some(m) if rows >= 10 && !m.matches.is_empty() => {
            crate::cmdmenu::menu_rows(m.matches.len()).min(POPUP_MAX)
        }
        _ => 0,
    }
}

/// Rows left for the item list.
pub(crate) fn list_height(p: &TodoPane, cols: u16, rows: u16) -> u16 {
    let cols = content(cols);

    rows.saturating_sub(composer::height(p, cols, rows) + popup_h(p, rows) + header_h(p, cols))
}

/// Column of a row's `✗`. One in from the gutter column ([`gutter`]) rather
/// than hard against it: a thumb drawn flush against the delete affordance
/// reads as a mark ON it.
pub(crate) fn del_col(cols: u16) -> u16 {
    cols.saturating_sub(3)
}

/// Rows item `it` occupies at this pane width.
pub(crate) fn item_h(it: &TodoItem, cols: u16, now_ms: u64, done_view: bool) -> u16 {
    let cols = content(cols);

    title_lines(it, cols, now_ms, done_view).len() as u16
        + u16::from(stacked(it, cols, now_ms, done_view))
}

/// Local calendar day of a done item's tick; `None` groups every legacy
/// (pre-stamp) tick into the one shared "earlier" bucket.
pub(crate) fn done_day(it: &TodoItem) -> Option<chrono::NaiveDate> {
    it.done_ms
        .and_then(duedate::from_epoch_ms)
        .map(|d| d.date())
}

/// Whether display row `di` opens a new day bucket in the done history —
/// its day-header row rides on this item, so every height sum stays a
/// per-item sum. Never true outside the view.
pub(crate) fn starts_day_group(
    items: &[TodoItem],
    done_view: bool,
    order: &[usize],
    di: usize,
) -> bool {
    done_view && (di == 0 || done_day(&items[order[di]]) != done_day(&items[order[di - 1]]))
}

/// Rows display entry `di` occupies: the item's wrapped title plus, in the
/// done view, the day header it opens. THE height truth for scroll, page
/// and click math — they must all sum this, or they disagree.
pub(crate) fn row_h(
    items: &[TodoItem],
    done_view: bool,
    order: &[usize],
    di: usize,
    cols: u16,
    now_ms: u64,
) -> u16 {
    let cols = content(cols);
    item_h(&items[order[di]], cols, now_ms, done_view)
        + u16::from(starts_day_group(items, done_view, order, di))
}
