//! The rows of the todo pane that are not items: the info row above the
//! list, the band headers between the rows, and the hint an empty list
//! shows instead of nothing.
//!
//! Split out of [`super::render`] for the line cap, along the line between
//! drawing an item and drawing everything around the items.
use crew_render::CellView;

use super::measure::{done_chip, done_chip_zone, filter_text};
use super::render::{cell, BOX_COL};
use super::{duedate, group, TodoPane};

/// The info row above the list: each active filter in its own tag colour
/// with the match count after them, and the done button right-aligned like
/// the pane card's own `[-]` and `[x]` — accent, because it is the one
/// thing on that row you can click.
pub(crate) fn cells(out: &mut Vec<CellView>, p: &TodoPane, cols: u16, shown: usize) {
    let t = crew_theme::theme();
    if let Some((project, who)) = filter_text(p) {
        let mut x = BOX_COL;
        for tag in [project, who].into_iter().flatten() {
            let fg = crew_theme::tag_color(tag.trim_start_matches(['@', '#']), t);
            x = crate::chatwidth::place_row(x, cols, tag.chars().map(|c| (c, ())), |x, c, ()| {
                out.push(cell(x, 0, c, fg, false))
            }) + 1;
        }
        let tail = format!("\u{b7} {shown} item{}", if shown == 1 { "" } else { "s" });
        crate::chatwidth::place_row(x, cols, tail.chars().map(|c| (c, ())), |x, c, ()| {
            out.push(cell(x, 0, c, t.text_muted, false))
        });
    }
    if let (Some(chip), Some((start, _))) = (done_chip(p), done_chip_zone(p, cols)) {
        let styled = chip.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(start, cols, styled, |x, c, ()| {
            out.push(cell(x, 0, c, crate::palette::accent(), false))
        });
    }
}

/// The band header riding on the item at `idx`: a day bucket in the done
/// history, or a `#assignee` group with its roll-up. The name wears its own
/// tag colour and the tally stays muted — the same split the info row uses,
/// so a band reads as one more filter you are looking through.
pub(crate) fn band(out: &mut Vec<CellView>, p: &TodoPane, idx: usize, row: u16, cols: u16) {
    let t = crew_theme::theme();
    let it = &p.items[idx];
    let (name, fg, tail) = match p.bands() {
        group::Bands::Days => {
            let today = duedate::now_local().date();
            let label = match super::measure::done_day(it) {
                Some(d) => duedate::day_label_naive(d, today),
                None => "earlier".to_string(),
            };
            (label, t.text_muted, None)
        }
        _ => {
            let now_ms = crate::chattime::unix_now_ms();
            let tally = group::tally(&p.items, p.filters(), group::who(it).as_deref(), now_ms);
            match &it.assignee {
                Some(w) => (format!("#{w}"), crew_theme::tag_color(w, t), Some(tally)),
                // The bucket for everything nobody has picked up. Named, not
                // blank: an unlabelled band at the foot of a grouped list
                // reads as the list having ended.
                None => ("unassigned".to_string(), t.text_muted, Some(tally)),
            }
        }
    };
    let x =
        crate::chatwidth::place_row(BOX_COL, cols, name.chars().map(|c| (c, ())), |x, c, ()| {
            out.push(cell(x, row, c, fg, true))
        });
    let Some(tail) = tail else { return };
    // Two columns of air, then the roll-up — dropped whole rather than
    // clipped when the pane is too narrow to hold it beside the name.
    let tail = format!("  {tail}");
    if x + crate::chatwidth::str_w(&tail) as u16 <= cols {
        crate::chatwidth::place_row(x, cols, tail.chars().map(|c| (c, ())), |x, c, ()| {
            out.push(cell(x, row, c, t.text_muted, false))
        });
    }
}

/// What an empty list says instead of nothing. An all-done list must not
/// read as a fresh one: with every item ticked there are no rows left, so
/// `H` (a list key) can't even be reached from here — Tab has nothing to
/// select. The way in from an empty pane is the command, so that is what
/// the hint names.
pub(crate) fn empty(out: &mut Vec<CellView>, p: &TodoPane, header: u16, lh: u16, cols: u16) {
    let t = crew_theme::theme();
    let done = super::item::done_count(&p.items, p.filters());
    let all_done = format!("all done \u{b7} {done} in the history");
    let none_here = p.filter.as_deref().map(|f| format!("nothing done in @{f}"));
    let hints: [&str; 2] = if p.done_view {
        [
            none_here.as_deref().unwrap_or("nothing done yet"),
            "tick an item on the list — it lands here",
        ]
    } else if done > 0 {
        [&all_done, "/todo done opens the log"]
    } else {
        [
            "no todos",
            "type one below — try: pay rent tomorrow 5pm @home #me",
        ]
    };
    for (i, hint) in hints.iter().enumerate() {
        let row = header + (lh / 2).saturating_sub(1) + i as u16;
        let hint = crate::chatwidth::clip_w(hint, usize::from(cols - BOX_COL));
        let styled = hint.chars().map(|c| (c, ()));
        crate::chatwidth::place_row(BOX_COL, cols, styled, |x, c, ()| {
            out.push(cell(x, row, c, t.text_muted, false))
        });
    }
}

#[cfg(test)]
#[path = "headrow_tests.rs"]
mod tests;
