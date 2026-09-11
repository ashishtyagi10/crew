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

/// The dim info row above the list: the filters' summary, the done button,
/// or both.
pub(crate) fn header_h(p: &TodoPane, cols: u16) -> u16 {
    // A button too wide for the pane isn't drawn, so it must not reserve the
    // row either — a narrow pane keeps every line for the list.
    u16::from(filter_text(p).is_some() || done_chip_zone(p, cols).is_some())
}

/// The header row's filter summary as `(@project, #who)` — whichever axes
/// are set, each drawn in its own tag colour. `None` when neither is.
pub(crate) fn filter_text(p: &TodoPane) -> Option<(Option<String>, Option<String>)> {
    let pair = (
        p.filter.as_ref().map(|f| format!("@{f}")),
        p.who.as_ref().map(|w| format!("#{w}")),
    );
    (pair.0.is_some() || pair.1.is_some()).then_some(pair)
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

/// The open tag pop-up's rows as menu items — the tag under its own sigil,
/// in the tag's colour — read by the draw and the hit-test alike, so the
/// card they size agrees.
pub(crate) fn tag_items(m: &super::tagmenu::TagMenu) -> Vec<crate::suggest::MenuItem> {
    let t = crew_theme::theme();
    let sigil = m.sigil;
    let item = move |tag: &String| crate::suggest::MenuItem {
        label: format!("{sigil}{tag}"),
        color: Some(crew_theme::tag_color(tag, t)),
        ..Default::default()
    };
    m.matches.iter().map(item).collect()
}

/// The pop-up card's width in a pane `cols` wide.
pub(crate) fn popup_w(m: &super::tagmenu::TagMenu, cols: u16) -> u16 {
    crate::popupplace::card_cols(crate::cmdrow::content_w(&tag_items(m)), cols)
}

/// The tag under content cell `(row, col)` when the pop-up card stands at
/// `top`, `ph` rows tall and `w` wide: the interior only — the border is
/// inert — mapped through the same scroll the list was drawn with.
pub(crate) fn tag_at(
    m: &super::tagmenu::TagMenu,
    (row, col): (u16, u16),
    (top, ph, w): (u16, u16, u16),
) -> Option<usize> {
    if ph < 3 || row <= top || row >= top + ph - 1 || col == 0 || col + 1 >= w {
        return None;
    }
    let first = crate::cmdmenu::offset_in(m.matches.len(), m.sel, usize::from(ph - 2));
    let i = first + usize::from(row - top - 1);
    (i < m.matches.len()).then_some(i)
}

/// The tag row of the open pop-up under content cell `(row, col)` of a
/// `cols × rows` pane, or `None` — the one geometry the click and the hover
/// read, so the row the hand points at is the row a press picks.
pub(crate) fn tag_under(p: &TodoPane, row: u16, col: u16, cols: u16, rows: u16) -> Option<usize> {
    let cols = super::render::content(cols);
    let ph = popup_h(p, rows);
    let m = p.tagmenu.as_ref().filter(|_| ph > 0)?;
    let top = rows - composer::height(p, cols, rows) - ph;
    tag_at(m, (row, col), (top, ph, popup_w(m, cols)))
}

/// Rows left for the item list.
pub(crate) fn list_height(p: &TodoPane, cols: u16, rows: u16) -> u16 {
    let cols = content(cols);

    rows.saturating_sub(composer::height(p, cols, rows) + popup_h(p, rows) + header_h(p, cols))
}

/// Column of a row's `✗`. Two in from the gutter column ([`gutter`]) rather
/// than hard against it: a thumb drawn flush against the delete affordance
/// reads as a mark ON it. The click zone is the glyph and the air after it
/// ([`del_zone`]) — never the gutter, which is only drawn when the list
/// overflows, which is exactly when a reader reaches for it.
pub(crate) fn del_col(cols: u16) -> u16 {
    cols.saturating_sub(3)
}

/// The columns a click deletes from: the `✗` and the one cell of air after it.
pub(crate) fn del_zone(cols: u16) -> std::ops::Range<u16> {
    del_col(cols)..del_col(cols) + 2
}

#[cfg(test)]
#[path = "delzone_tests.rs"]
mod delzone_tests;

#[cfg(test)]
#[path = "emptyhint_tests.rs"]
mod emptyhint_tests;

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

/// Rows display entry `di` occupies: the item's wrapped title plus the band
/// header it opens, if any ([`super::group::starts`]). THE height truth for
/// scroll, page and click math — they must all sum this, or they disagree.
pub(crate) fn row_h(p: &TodoPane, order: &[usize], di: usize, cols: u16, now_ms: u64) -> u16 {
    let cols = content(cols);
    item_h(&p.items[order[di]], cols, now_ms, p.done_view)
        + u16::from(super::group::starts(p, order, di))
}
